// Applies a stored theme before the page paints (a blocking script, so there's no flash).
// Inlined into each page at build time (scripts/inline-css.mjs) and allowed by its hash in the CSP.
// Logic mirrors src/scripts/theme.ts.
(function () {
  try {
    var t = window.localStorage.getItem("fettle-theme");
    if (t !== "light" && t !== "dark" && t !== "oled") return;
    document.documentElement.setAttribute("data-theme", t);
    var colours = { light: "#E8EDE7", dark: "#1F6A45", oled: "#000000" };
    var metas = document.querySelectorAll('meta[name="theme-color"]');
    for (var i = 0; i < metas.length; i++) metas[i].setAttribute("content", colours[t]);
  } catch (e) {
    // Storage blocked: follow the device.
  }
})();

// Arriving from another Fettle page (through a page transition): the page shouldn't open with its
// own entrance on top of that, so the homepage's hero stays still (site.css, data-arrival).
addEventListener("pagereveal", function (e) {
  if (e.viewTransition) document.documentElement.setAttribute("data-arrival", "page");
});
