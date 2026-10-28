// Aplica el tema antes del primer render para evitar el parpadeo.
(function () {
  try {
    var t = localStorage.getItem("pluma.theme");
    var themes = ["light", "dark", "sepia", "paper"];
    if (themes.indexOf(t) === -1) t = "system";
    if (t === "system") {
      t = window.matchMedia("(prefers-color-scheme: dark)").matches
        ? "dark"
        : "light";
    }
    document.documentElement.dataset.theme = t;
  } catch (e) {}
})();
