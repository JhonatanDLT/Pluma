//! Gestión del runtime de ONNX Runtime (biblioteca dinámica compartida).
//!
//! Tanto `fastembed` (embeddings) como `gliner2-rs` (NER) se apoyan en ONNX
//! Runtime a través de `ort`. En este árbol de dependencias `ort` se compila en
//! modo `load-dynamic`: la biblioteca nativa se carga en tiempo de ejecución
//! desde la variable `ORT_DYLIB_PATH` (o desde un nombre por defecto junto al
//! ejecutable).
//!
//! Para no obligar al usuario a instalar ONNX Runtime a mano, este módulo
//! descarga y extrae la biblioteca dinámica una sola vez —con permiso explícito
//! del usuario— desde los releases oficiales de ONNX Runtime, y fija
//! `ORT_DYLIB_PATH` para que `ort` la encuentre. Es un componente **opt-in**:
//! hasta que el runtime no está instalado, los motores pesados no se cargan y
//! el análisis cae a las reglas locales.
//!
//! > Nota: se usan los releases de GitHub (y no el CDN de `ort-sys`) porque este
//! > último solo publica la librería **estática** (`libonnxruntime.a`) para macOS,
//! > mientras que `load-dynamic` necesita la `.dylib`/`.so`/`.dll`.

use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use flate2::read::GzDecoder;
use sha2::{Digest, Sha256};

use crate::error::{AppError, Result};

/// Nombre de la biblioteca nativa según la plataforma.
fn dylib_filename() -> &'static str {
    if cfg!(target_os = "macos") {
        "libonnxruntime.dylib"
    } else if cfg!(target_os = "windows") {
        "onnxruntime.dll"
    } else {
        "libonnxruntime.so"
    }
}

/// Formato del paquete descargado.
enum ArchiveKind {
    Tgz,
    Zip,
}

/// Distribución oficial de ONNX Runtime 1.28 (compatible con `ort` rc.13).
struct Distribution {
    url: &'static str,
    sha256: &'static str,
    kind: ArchiveKind,
}

fn distribution() -> Result<Distribution> {
    match (std::env::consts::OS, std::env::consts::ARCH) {
        ("macos", "aarch64") => Ok(Distribution {
            url: "https://github.com/microsoft/onnxruntime/releases/download/v1.28.0/onnxruntime-osx-arm64-1.28.0.tgz",
            sha256: "1268b359718099bde2cedb55787f182a130067bc4f31e8c88478c445b850d3d8",
            kind: ArchiveKind::Tgz,
        }),
        ("linux", "x86_64") => Ok(Distribution {
            url: "https://github.com/microsoft/onnxruntime/releases/download/v1.28.0/onnxruntime-linux-x64-1.28.0.tgz",
            sha256: "a3e1b79d7bb1bf09696ce675f49e4064e6c81f6202b8225624fff0e93f8d6407",
            kind: ArchiveKind::Tgz,
        }),
        ("linux", "aarch64") => Ok(Distribution {
            url: "https://github.com/microsoft/onnxruntime/releases/download/v1.28.0/onnxruntime-linux-aarch64-1.28.0.tgz",
            sha256: "e15ff8b5d85afe6c144d97c6fd432254bf76a219daaf17658087d6ecb3e8f0bb",
            kind: ArchiveKind::Tgz,
        }),
        ("windows", "x86_64") => Ok(Distribution {
            url: "https://github.com/microsoft/onnxruntime/releases/download/v1.28.0/onnxruntime-win-x64-1.28.0.zip",
            sha256: "abef733dacbe2f571547a7150b479b5cb9cc0df22f96c24983a42cadb1b4f8bc",
            kind: ArchiveKind::Zip,
        }),
        (os, arch) => Err(AppError::Validation(format!(
            "no hay runtime de ONNX Runtime precompilado para {arch}-{os}; \
             instálalo manualmente y define ORT_DYLIB_PATH"
        ))),
    }
}

/// Directorio donde se guarda el runtime dentro del directorio de datos.
pub fn runtime_dir(data_dir: &Path) -> PathBuf {
    data_dir.join("runtime")
}

/// Ruta esperada de la biblioteca nativa.
pub fn dylib_path(data_dir: &Path) -> PathBuf {
    runtime_dir(data_dir).join(dylib_filename())
}

/// Prepara el runtime al arrancar: si ya existe una biblioteca (en el directorio
/// de datos o junto al ejecutable) y `ORT_DYLIB_PATH` no está definido, lo fija.
/// Debe ejecutarse **antes** de cargar cualquier modelo.
pub fn prepare(data_dir: &Path) {
    if std::env::var_os("ORT_DYLIB_PATH").is_some_and(|v| !v.is_empty()) {
        return;
    }

    let candidates = [
        Some(dylib_path(data_dir)),
        std::env::current_exe()
            .ok()
            .and_then(|exe| exe.parent().map(|p| p.join(dylib_filename()))),
    ];

    for candidate in candidates.into_iter().flatten() {
        if candidate.is_file() {
            // Seguro: se ejecuta al arrancar, antes de que `ort` inicialice su
            // entorno y sin concurrencia con otros accesos a variables de entorno.
            unsafe { std::env::set_var("ORT_DYLIB_PATH", &candidate) };
            return;
        }
    }
}

/// Indica si el runtime está disponible (instalado en la app, junto al
/// ejecutable o definido por el usuario).
pub fn is_available(data_dir: &Path) -> bool {
    std::env::var_os("ORT_DYLIB_PATH").is_some_and(|v| !v.is_empty())
        || dylib_path(data_dir).is_file()
}

/// Descarga, verifica y extrae el runtime. Devuelve la ruta de la biblioteca.
pub fn install(data_dir: &Path) -> Result<PathBuf> {
    let dist = distribution()?;
    let dir = runtime_dir(data_dir);
    std::fs::create_dir_all(&dir)?;

    let archive_path = dir.join(match dist.kind {
        ArchiveKind::Tgz => "onnxruntime.tar.gz",
        ArchiveKind::Zip => "onnxruntime.zip",
    });

    download_verified(dist.url, dist.sha256, &archive_path)?;
    match dist.kind {
        ArchiveKind::Tgz => extract_tgz(&archive_path, &dir)?,
        ArchiveKind::Zip => extract_zip(&archive_path, &dir)?,
    }
    let _ = std::fs::remove_file(&archive_path);

    let dylib = dylib_path(data_dir);
    if !dylib.is_file() {
        return Err(AppError::Internal(format!(
            "el archivo {} no se encontró tras la extracción",
            dylib.display()
        )));
    }

    // Fija la variable para el proceso actual, de modo que el usuario pueda
    // continuar sin reiniciar la aplicación. Seguro: en este punto `ort` aún no
    // ha cargado su entorno (el motor pesado se crea después).
    unsafe { std::env::set_var("ORT_DYLIB_PATH", &dylib) };
    Ok(dylib)
}

/// Descarga `url` a `dest` verificando su hash SHA-256 en el proceso.
fn download_verified(url: &str, expected_sha256: &str, dest: &Path) -> Result<()> {
    let response = ureq::get(url)
        .call()
        .map_err(|e| AppError::Internal(format!("no se pudo descargar el runtime: {e}")))?;

    let mut reader = response.into_reader();
    let mut file = std::fs::File::create(dest)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];

    loop {
        let read = reader
            .read(&mut buffer)
            .map_err(|e| AppError::Internal(format!("error al descargar el runtime: {e}")))?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
        file.write_all(&buffer[..read])?;
    }
    file.flush()?;

    let digest = hasher.finalize();
    let actual = hex(&digest);
    if actual != expected_sha256 {
        let _ = std::fs::remove_file(dest);
        return Err(AppError::Internal(format!(
            "el runtime descargado no coincide con el hash esperado \
             (obtenido {actual}, esperado {expected_sha256})"
        )));
    }
    Ok(())
}

/// Extrae las bibliotecas nativas de un `.tar.gz`, aplanando la estructura.
fn extract_tgz(archive_path: &Path, dir: &Path) -> Result<()> {
    let file = std::fs::File::open(archive_path)?;
    let decoder = GzDecoder::new(file);
    let mut archive = tar::Archive::new(decoder);

    for entry in archive
        .entries()
        .map_err(|e| AppError::Internal(format!("archivo del runtime inválido: {e}")))?
    {
        let mut entry = entry.map_err(|e| AppError::Internal(format!("error de lectura: {e}")))?;
        let path = entry
            .path()
            .map_err(|e| AppError::Internal(format!("ruta inválida: {e}")))?
            .into_owned();

        if !is_library(&path) {
            continue;
        }

        let basename = path.file_name().ok_or_else(|| {
            AppError::Internal(format!("entrada sin nombre de archivo: {}", path.display()))
        })?;
        entry
            .unpack(dir.join(basename))
            .map_err(|e| AppError::Internal(format!("error al extraer {}: {e}", path.display())))?;
    }
    Ok(())
}

/// Extrae las bibliotecas nativas de un `.zip`, aplanando la estructura.
fn extract_zip(archive_path: &Path, dir: &Path) -> Result<()> {
    let file = std::fs::File::open(archive_path)?;
    let mut archive = zip::ZipArchive::new(file)
        .map_err(|e| AppError::Internal(format!("archivo del runtime inválido: {e}")))?;

    for i in 0..archive.len() {
        let mut entry = archive
            .by_index(i)
            .map_err(|e| AppError::Internal(format!("error de lectura: {e}")))?;
        let name = entry
            .name()
            .map_err(|e| AppError::Internal(format!("error de lectura: {e}")))?;
        let path = PathBuf::from(name.as_ref());
        if !is_library(&path) {
            continue;
        }

        let basename = path.file_name().ok_or_else(|| {
            AppError::Internal(format!("entrada sin nombre de archivo: {}", path.display()))
        })?;
        let mut out = std::fs::File::create(dir.join(basename))?;
        std::io::copy(&mut entry, &mut out)?;
    }
    Ok(())
}

/// Decide si una entrada del paquete es una biblioteca nativa de ONNX Runtime.
/// Excluye símbolos de depuración (`.dSYM`) y bibliotecas estáticas (`.a`).
///
/// En Linux los nombres van versionados al final (`libonnxruntime.so.1.28.0`),
/// por eso se busca la subcadena `.so` en lugar de un sufijo exacto.
fn is_library(path: &Path) -> bool {
    if path.to_string_lossy().contains(".dSYM") {
        return false;
    }
    let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
        return false;
    };
    let unix_dylib =
        name.starts_with("libonnxruntime") && (name.contains(".so") || name.contains(".dylib"));
    let win_dll = name.starts_with("onnxruntime") && name.ends_with(".dll");
    unix_dylib || win_dll
}

fn hex(bytes: &[u8]) -> String {
    const HEX: &[u8] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for &b in bytes {
        out.push(HEX[(b >> 4) as usize] as char);
        out.push(HEX[(b & 0xf) as usize] as char);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use flate2::Compression;
    use flate2::write::GzEncoder;

    #[test]
    fn extracts_libraries_and_flattens() {
        let dir = std::env::temp_dir().join(format!("pluma-runtime-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        // Construye un tar.gz con una dylib, un `.so` versionado y un symlink.
        let archive_path = dir.join("test.tar.gz");
        let file = std::fs::File::create(&archive_path).unwrap();
        let encoder = GzEncoder::new(file, Compression::default());
        let mut builder = tar::Builder::new(encoder);

        let mut regular = tar::Header::new_gnu();
        regular.set_size(4);
        regular.set_mode(0o644);
        builder
            .append_data(&mut regular, "lib/libonnxruntime.dylib", &b"data"[..])
            .unwrap();

        let mut versioned = tar::Header::new_gnu();
        versioned.set_size(4);
        versioned.set_mode(0o644);
        builder
            .append_data(&mut versioned, "lib/libonnxruntime.so.1.28.0", &b"data"[..])
            .unwrap();

        let mut link = tar::Header::new_gnu();
        link.set_entry_type(tar::EntryType::Symlink);
        link.set_size(0);
        builder
            .append_link(
                &mut link,
                "lib/libonnxruntime.so",
                "libonnxruntime.so.1.28.0",
            )
            .unwrap();

        builder.into_inner().unwrap().finish().unwrap();

        let out = dir.join("out");
        std::fs::create_dir_all(&out).unwrap();
        extract_tgz(&archive_path, &out).unwrap();

        assert!(out.join("libonnxruntime.dylib").is_file());
        assert!(out.join("libonnxruntime.so.1.28.0").is_file());
        // El symlink aplanado debe resolver a su destino.
        assert!(out.join("libonnxruntime.so").is_file());

        let _ = std::fs::remove_dir_all(&dir);
    }
}
