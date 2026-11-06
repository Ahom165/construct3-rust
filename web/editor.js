/* C3R Editor — UI web originale inspirée des éditeurs de jeux 2D modernes */
"use strict";

var S = {
  project: null,
  warnings: [],
  sel: null,            // {kind:'instance'|'type'|'asset'|'sheet'|'layout', ...}
  tab: "start",         // start | layout:<name> | sheet:<name>
  openTabs: ["start"],
  zoom: 1,
  playerSrc: null,
  imgCache: {}
};

function $(s) { return document.querySelector(s); }
function el(tag, cls, txt) {
  var e = document.createElement(tag);
  if (cls) e.className = cls;
  if (txt != null) e.textContent = txt;
  return e;
}
function api(method, url, body) {
  return fetch(url, { method: method, body: body }).then(function (r) { return r.json(); });
}
function findLayout(name) { return S.project.layouts.filter(function (l) { return l.name === name; })[0] || null; }
function findSheet(name) { return S.project.event_sheets.filter(function (s) { return s.name === name; })[0] || null; }
function findType(name) { return S.project.object_types.filter(function (t) { return t.name === name; })[0] || null; }
function assetById(id) { return S.project.assets.filter(function (a) { return a.id === id; })[0] || null; }
function assetImage(idOrName) {
  var a = assetById(idOrName);
  if (!a) a = S.project.assets.filter(function (x) { return x.name === idOrName; })[0];
  if (!a || a.kind !== "image") return null;
  if (!S.imgCache[a.id]) {
    var im = new Image();
    im.onload = function () { drawLayoutCanvas(); };
    im.src = a.data;
    S.imgCache[a.id] = im;
  }
  return S.imgCache[a.id];
}
function nextUid(prefix) {
  var m = 1;
  S.project.object_types.forEach(function (t) { if (t.uid >= m) m = t.uid + 1; });
  return prefix === undefined ? m : prefix + m;
}
function status(msg) { $("#status-left").textContent = msg; }
function toast(msg, cls) {
  var t = el("div", "toast" + (cls ? " " + cls : ""), msg);
  $("#toasts").appendChild(t);
  setTimeout(function () { t.remove(); }, 5200);
}

/* ------------------------------------------------------------ catalogue */

var CONDS = [
  { id: "on_start_of_layout", cat: "Système", label: "On start of layout", params: [] },
  { id: "every_tick", cat: "Système", label: "Every tick", params: [] },
  { id: "every_x_seconds", cat: "Système", label: "Every X seconds", params: [{ k: "Secondes", t: "number" }] },
  { id: "compare_global_var", cat: "Système", label: "Compare global variable", params: [{ k: "Variable", t: "gvar" }, { k: "Comparaison", t: "select", opts: ["==", "!=", "<", "<=", ">", ">="] }, { k: "Valeur", t: "number" }] },
  { id: "random_chance", cat: "Système", label: "Random chance (%)", params: [{ k: "% chance", t: "number" }] },
  { id: "key_pressed", cat: "Clavier", label: "On key pressed", params: [{ k: "Touche", t: "key" }] },
  { id: "key_down", cat: "Clavier", label: "Is key down", params: [{ k: "Touche", t: "key" }] },
  { id: "key_released", cat: "Clavier", label: "On key released", params: [{ k: "Touche", t: "key" }] },
  { id: "on_collision_with", cat: "Objet", label: "On collision with another object", obj: true, params: [{ k: "Objet", t: "object" }] },
  { id: "on_object_clicked", cat: "Objet", label: "On object clicked", obj: true, params: [] },
  { id: "on_created", cat: "Objet", label: "On created", obj: true, params: [] },
  { id: "on_destroyed", cat: "Objet", label: "On destroyed", obj: true, params: [] },
  { id: "compare_instance_var", cat: "Objet", label: "Compare instance variable", obj: true, params: [{ k: "Variable", t: "text" }, { k: "Comparaison", t: "select", opts: ["==", "!=", "<", "<=", ">", ">="] }, { k: "Valeur", t: "number" }] },
  { id: "compare_x", cat: "Objet", label: "Compare X", obj: true, params: [{ k: "Comparaison", t: "select", opts: ["==", "!=", "<", "<=", ">", ">="] }, { k: "Valeur", t: "number" }] },
  { id: "compare_y", cat: "Objet", label: "Compare Y", obj: true, params: [{ k: "Comparaison", t: "select", opts: ["==", "!=", "<", "<=", ">", ">="] }, { k: "Valeur", t: "number" }] },
  { id: "is_on_screen", cat: "Objet", label: "Is on screen", obj: true, params: [] },
  { id: "on_timer", cat: "Objet", label: "On timer", obj: true, params: [{ k: "Nom du timer", t: "text" }] }
];
var ACTIONS = [
  { id: "create_object", cat: "Système", label: "Create object", params: [{ k: "Objet", t: "object" }, { k: "Layer", t: "layer" }, { k: "X", t: "number" }, { k: "Y", t: "number" }] },
  { id: "set_global_var", cat: "Système", label: "Set global variable", params: [{ k: "Variable", t: "gvar" }, { k: "Opération", t: "select", opts: ["set", "add", "sub"] }, { k: "Valeur", t: "number" }] },
  { id: "go_to_layout", cat: "Système", label: "Go to layout", params: [{ k: "Layout", t: "layout" }] },
  { id: "restart_layout", cat: "Système", label: "Restart layout", params: [] },
  { id: "play_sound", cat: "Audio", label: "Play sound", params: [{ k: "Son", t: "asset-audio" }, { k: "Boucle", t: "select", opts: ["0", "1"] }, { k: "Volume (0-1)", t: "number" }] },
  { id: "spawn", cat: "Objet", label: "Spawn another object", obj: true, params: [{ k: "Objet", t: "object" }, { k: "Layer", t: "layer" }] },
  { id: "destroy", cat: "Objet", label: "Destroy", obj: true, params: [] },
  { id: "set_x", cat: "Objet", label: "Set X", obj: true, params: [{ k: "X", t: "number" }] },
  { id: "set_y", cat: "Objet", label: "Set Y", obj: true, params: [{ k: "Y", t: "number" }] },
  { id: "set_position", cat: "Objet", label: "Set position", obj: true, params: [{ k: "X", t: "number" }, { k: "Y", t: "number" }] },
  { id: "set_angle", cat: "Objet", label: "Set angle", obj: true, params: [{ k: "Degrés", t: "number" }] },
  { id: "set_width", cat: "Objet", label: "Set width", obj: true, params: [{ k: "Largeur", t: "number" }] },
  { id: "set_height", cat: "Objet", label: "Set height", obj: true, params: [{ k: "Hauteur", t: "number" }] },
  { id: "set_opacity", cat: "Objet", label: "Set opacity", obj: true, params: [{ k: "Opacité (0-100)", t: "number" }] },
  { id: "set_z", cat: "Objet", label: "Set Z order", obj: true, params: [{ k: "Z", t: "number" }] },
  { id: "set_instance_var", cat: "Objet", label: "Set instance variable", obj: true, params: [{ k: "Variable", t: "text" }, { k: "Opération", t: "select", opts: ["set", "add", "sub"] }, { k: "Valeur", t: "number" }] },
  { id: "set_text", cat: "Objet", label: "Set text", obj: true, params: [{ k: "Texte ({var} accepté)", t: "text" }] },
  { id: "set_font_size", cat: "Objet", label: "Set font size", obj: true, params: [{ k: "Taille", t: "number" }] },
  { id: "set_animation", cat: "Objet", label: "Set animation", obj: true, params: [{ k: "Animation", t: "text" }] },
  { id: "set_animation_frame", cat: "Objet", label: "Set animation frame", obj: true, params: [{ k: "Frame", t: "number" }] },
  { id: "move_forward", cat: "Objet", label: "Move forward", obj: true, params: [{ k: "Pixels", t: "number" }] },
  { id: "scroll_to", cat: "Objet", label: "Scroll to object", obj: true, params: [] },
  { id: "set_eightdir_velocity", cat: "Objet", label: "Set 8 direction velocity", obj: true, params: [{ k: "VX", t: "number" }, { k: "VY", t: "number" }] },
  { id: "set_bullet_speed", cat: "Objet", label: "Set bullet speed", obj: true, params: [{ k: "Vitesse", t: "number" }] },
  { id: "start_timer", cat: "Objet", label: "Start timer", obj: true, params: [{ k: "Nom", t: "text" }, { k: "Secondes", t: "number" }] },
  { id: "pin_to", cat: "Objet", label: "Pin to object", obj: true, params: [{ k: "Objet", t: "object" }] }
];
var KEYS = ["left", "right", "up", "down", "space", "enter", "shift", "control", "w", "a", "s", "d", "e", "q", "z", "x", "esc"];

function condDef(id) { return CONDS.filter(function (c) { return c.id === id; })[0] || { id: id, label: id, params: [], cat: "?" }; }
function actDef(id) { return ACTIONS.filter(function (c) { return c.id === id; })[0] || { id: id, label: id, params: [], cat: "?" }; }
function fmtParams(params) {
  return (params || []).map(function (p) {
    if (p === null || p === undefined) return "";
    if (typeof p === "object") return JSON.stringify(p);
    return String(p);
  }).join(", ");
}

/* ------------------------------------------------------------ boot */

async function boot() {
  S.playerSrc = await fetch("/s/player.js").then(function (r) { return r.text(); });
  await refresh();
  buildUI();
  (S.warnings || []).forEach(function (w) { toast(w, "warn"); });
}
async function refresh() {
  var st = await api("GET", "/api/state");
  S.project = st.project;
  S.warnings = st.warnings || [];
}
function saveProject() { return api("POST", "/api/project", JSON.stringify(S.project)); }

/* ------------------------------------------------------------ menus */

var MENUS = {
  "Fichier": [
    ["Nouveau projet", newProject, "Ctrl+N"],
    ["Ouvrir un .c3p…", openC3pDialog, "Ctrl+O"],
    ["Importer des fichiers…", importDialog, "Ctrl+I"],
    "-",
    ["Enregistrer en .c3p", saveC3p, "Ctrl+S"],
    ["Exporter le jeu (HTML5)", exportHtml, "Ctrl+E"]
  ],
  "Projet": [
    ["Ajouter un Sprite", addSprite, null],
    ["Ajouter un Tiled Background", addTiled, null],
    ["Ajouter un Texte", addText, null],
    "-",
    ["Nouveau layout", addLayout, null],
    ["Nouvel event sheet", addSheet, null],
    ["Nouvelle variable globale", addGlobalVar, null]
  ],
  "Affichage": [
    ["Zoom avant", zoomIn, "+"],
    ["Zoom arrière", zoomOut, "-"],
    ["Zoom 100 %", zoomReset, null]
  ],
  "Aide": [
    ["À propos de C3R", about, null]
  ]
};

function buildMenus() {
  var nav = $("#menus");
  nav.innerHTML = "";
  Object.keys(MENUS).forEach(function (name) {
    var mi = el("div", "menu-item", name);
    var drop = el("div", "menu-drop");
    MENUS[name].forEach(function (entry) {
      if (entry === "-") { drop.appendChild(el("div", "menu-sep")); return; }
      var e = el("div", "menu-entry");
      e.appendChild(el("span", null, entry[0]));
      if (entry[2]) e.appendChild(el("span", "hk", entry[2]));
      e.addEventListener("click", function () { closeMenus(); entry[1](); });
      drop.appendChild(e);
    });
    mi.appendChild(drop);
    mi.addEventListener("click", function (ev) {
      var was = mi.classList.contains("open");
      closeMenus();
      if (!was) mi.classList.add("open");
      ev.stopPropagation();
    });
    nav.appendChild(mi);
  });
  document.addEventListener("click", closeMenus);
}
function closeMenus() { document.querySelectorAll(".menu-item.open").forEach(function (m) { m.classList.remove("open"); }); }

/* ------------------------------------------------------------ ribbon */

var RIBBON = {
  "Accueil": [
    [function () { var g = el("div", "rb-group"); RIBBON_Home1.forEach(function (b) { g.appendChild(rbBtn(b)); }); return g; }],
    [function () { var g = el("div", "rb-group"); RIBBON_Home2.forEach(function (b) { g.appendChild(rbBtn(b)); }); return g; }]
  ]
};
var RIBBON_Home1 = [
  { label: "Nouveau", color: "#5a8fd8", glyph: "N", fn: newProject },
  { label: "Ouvrir .c3p", color: "#8f6fd8", glyph: "O", fn: openC3pDialog },
  { label: "Importer", color: "#d89f3f", glyph: "I", fn: importDialog }
];
var RIBBON_Home2 = [
  { label: "Enregistrer", color: "#66c04a", glyph: "S", fn: saveC3p },
  { label: "Exporter", color: "#4ab8c0", glyph: "E", fn: exportHtml },
  { label: "Play", color: "#e05555", glyph: "\u25B6", fn: play }
];
function rbBtn(b) {
  var btn = el("button", "rb-btn");
  var ico = el("div", "rb-ico", b.glyph);
  ico.style.background = b.color;
  btn.appendChild(ico);
  btn.appendChild(el("span", "lbl", b.label));
  btn.addEventListener("click", b.fn);
  return btn;
}
function buildRibbon() {
  var tabs = $("#ribbon-tabs"), body = $("#ribbon-body");
  tabs.innerHTML = ""; body.innerHTML = "";
  Object.keys(RIBBON).forEach(function (name, i) {
    var t = el("div", "ribbon-tab" + (i === 0 ? " active" : ""), name);
    t.addEventListener("click", function () {
      document.querySelectorAll(".ribbon-tab").forEach(function (x) { x.classList.remove("active"); });
      t.classList.add("active");
      buildRibbonBody(name);
    });
    tabs.appendChild(t);
  });
  buildRibbonBody(Object.keys(RIBBON)[0]);
}
function buildRibbonBody(name) {
  var body = $("#ribbon-body");
  body.innerHTML = "";
  RIBBON[name].forEach(function (entry) { body.appendChild(entry[0]()); });
}

/* ------------------------------------------------------------ tree */

function buildTree() {
  var tree = $("#tree");
  tree.innerHTML = "";
  var head = el("div", "tree-head");
  head.appendChild(el("span", null, S.project.name));
  tree.appendChild(head);

  // Objets
  var sec = el("div", "tree-section");
  var h = el("div", "tree-head", "Objets");
  var addB = el("div", "add", "+"); addB.title = "Ajouter un objet";
  addB.addEventListener("click", addObjectDialog);
  h.appendChild(addB);
  sec.appendChild(h);
  S.project.object_types.forEach(function (t) {
    if (t.plugin === "system") return;
    var it = el("div", "tree-item" + (S.sel && S.sel.kind === "type" && S.sel.name === t.name ? " selected" : ""));
    it.appendChild(el("span", null, t.name));
    it.appendChild(el("span", "kind", t.plugin));
    it.addEventListener("click", function () { S.sel = { kind: "type", name: t.name }; buildTree(); buildProps(); });
    sec.appendChild(it);
  });
  tree.appendChild(sec);

  // Layouts
  sec = el("div", "tree-section");
  h = el("div", "tree-head", "Layouts");
  addB = el("div", "add", "+"); addB.title = "Nouveau layout";
  addB.addEventListener("click", addLayout);
  h.appendChild(addB);
  sec.appendChild(h);
  S.project.layouts.forEach(function (l) {
    var it = el("div", "tree-item" + (S.tab === "layout:" + l.name ? " selected" : ""));
    it.appendChild(el("span", null, l.name));
    it.appendChild(el("span", "kind", l.width + "\u00D7" + l.height));
    it.addEventListener("click", function () { openTab("layout:" + l.name); });
    sec.appendChild(it);
  });
  tree.appendChild(sec);

  // Event sheets
  sec = el("div", "tree-section");
  h = el("div", "tree-head", "Event sheets");
  addB = el("div", "add", "+"); addB.title = "Nouvel event sheet";
  addB.addEventListener("click", addSheet);
  h.appendChild(addB);
  sec.appendChild(h);
  S.project.event_sheets.forEach(function (s) {
    var it = el("div", "tree-item" + (S.tab === "sheet:" + s.name ? " selected" : ""));
    it.appendChild(el("span", null, s.name));
    it.appendChild(el("span", "kind", s.events.length + " \u00E9v."));
    it.addEventListener("click", function () { openTab("sheet:" + s.name); });
    sec.appendChild(it);
  });
  tree.appendChild(sec);

  // Assets
  sec = el("div", "tree-section");
  h = el("div", "tree-head", "Ressources (" + S.project.assets.length + ")");
  addB = el("div", "add", "+"); addB.title = "Importer des fichiers";
  addB.addEventListener("click", importDialog);
  h.appendChild(addB);
  sec.appendChild(h);
  if (!S.project.assets.length) sec.appendChild(el("div", "tree-empty", "Glissez-déposez n'importe quel fichier"));
  S.project.assets.forEach(function (a) {
    var it = el("div", "tree-item" + (S.sel && S.sel.kind === "asset" && S.sel.id === a.id ? " selected" : ""));
    it.appendChild(el("span", null, a.name));
    it.appendChild(el("span", "kind", a.kind));
    it.addEventListener("click", function () { S.sel = { kind: "asset", id: a.id }; buildTree(); buildProps(); });
    sec.appendChild(it);
  });
  tree.appendChild(sec);
}

/* ------------------------------------------------------------ tabs & views */

function openTab(name) {
  if (S.openTabs.indexOf(name) < 0) S.openTabs.push(name);
  S.tab = name;
  buildTabs();
  buildView();
  buildTree();
}
function closeTab(name) {
  S.openTabs = S.openTabs.filter(function (t) { return t !== name; });
  if (S.tab === name) S.tab = S.openTabs[S.openTabs.length - 1] || "start";
  buildTabs(); buildView(); buildTree();
}
function tabTitle(name) {
  if (name === "start") return "Accueil";
  if (name.indexOf("layout:") === 0) return "Layout : " + name.slice(7);
  if (name.indexOf("sheet:") === 0) return "\u00C9v\u00E9nements : " + name.slice(6);
  return name;
}
function buildTabs() {
  var tabs = $("#tabs");
  tabs.innerHTML = "";
  S.openTabs.forEach(function (name) {
    var t = el("div", "tab" + (S.tab === name ? " active" : ""));
    t.appendChild(el("span", null, tabTitle(name)));
    if (name !== "start") {
      var x = el("span", "close", "\u00D7");
      x.addEventListener("click", function (e) { e.stopPropagation(); closeTab(name); });
      t.appendChild(x);
    }
    t.addEventListener("click", function () { S.tab = name; buildTabs(); buildView(); buildTree(); });
    tabs.appendChild(t);
  });
}
function buildView() {
  var v = $("#views");
  v.innerHTML = "";
  if (S.tab === "start") buildStartPage(v);
  else if (S.tab.indexOf("layout:") === 0) buildLayoutView(v, S.tab.slice(7));
  else if (S.tab.indexOf("sheet:") === 0) buildSheetView(v, S.tab.slice(6));
}

/* ---- start page ---- */
function buildStartPage(v) {
  var w = el("div", "start-hero");
  var h1 = el("h1");
  h1.appendChild(document.createTextNode("Bienvenue dans "));
  var b = el("b", null, "C3R");
  h1.appendChild(b);
  w.appendChild(h1);
  w.appendChild(el("p", "sub", "Construct 3 Rust Edition \u2014 \u00E9diteur de jeux 2D, import universel, moteur HTML5 int\u00E9gr\u00E9."));
  var dz = el("div", "dropzone");
  dz.appendChild(el("div", "big", "D\u00E9posez ici N'IMPORTE QUEL fichier"));
  dz.appendChild(el("div", null, " "));
  dz.appendChild(el("small", null, ".c3p \u2192 projet \u00B7 images \u2192 sprites \u00B7 sons \u2192 audio \u00B7 zip \u2192 extraction \u00B7 tout le reste \u2192 ressource"));
  dz.addEventListener("click", importDialog);
  w.appendChild(dz);
  var cards = el("div", "start-cards");
  [
    ["Ouvrir un .c3p", "Chargez un projet existant", openC3pDialog],
    ["Nouveau projet", "Partez d'une page blanche", newProject],
    ["Ouvrir la d\u00E9mo", "Plateforme d'exemple (demo-platformer.c3p)", openDemo],
    ["Exporter HTML5", "Un seul fichier .html jouable", exportHtml]
  ].forEach(function (c) {
    var card = el("div", "card");
    card.appendChild(el("b", null, c[0]));
    card.appendChild(el("span", null, c[1]));
    card.addEventListener("click", c[2]);
    cards.appendChild(card);
  });
  w.appendChild(cards);
  w.appendChild(el("p", "sub", " "));
  w.appendChild(el("small", null, "Astuce : F5 pour jouer \u00B7 Ctrl+I pour importer \u00B7 glisser-d\u00E9poser fonctionne partout"));
  v.appendChild(w);
}
async function openDemo() {
  try {
    var r = await fetch("/demo.c3p");
    if (!r.ok) { toast("Demo indisponible : placez demo-platformer.c3p \u00E0 c\u00F4t\u00E9 du serveur", "err"); return; }
    var buf = await r.arrayBuffer();
    await openC3pBytes(buf);
  } catch (e) { toast("Erreur : " + e, "err"); }
}

/* ---- layout view ---- */
function buildLayoutView(v, lname) {
  var L = findLayout(lname);
  if (!L) return;
  var wrap = el("div", "layout-wrap");
  var zb = el("div", "zoombar");
  [["\u2212", zoomOut], ["100 %", zoomReset], ["+", zoomIn]].forEach(function (z) {
    var b = el("button", null, z[0]);
    b.addEventListener("click", z[1]);
    zb.appendChild(b);
  });
  wrap.appendChild(zb);
  var cv = el("canvas");
  cv.id = "layoutcanvas";
  cv.width = Math.floor(L.width * S.zoom);
  cv.height = Math.floor(L.height * S.zoom);
  wrap.appendChild(cv);
  v.appendChild(wrap);
  cv.dataset.layout = lname;

  var drag = null;
  cv.addEventListener("mousedown", function (e) {
    var r = cv.getBoundingClientRect();
    var x = (e.clientX - r.left) / S.zoom, y = (e.clientY - r.top) / S.zoom;
    var hit = pickInstance(L, x, y);
    S.sel = hit ? { kind: "instance", layout: lname, uid: hit.uid } : null;
    buildProps(); buildTree();
    if (hit) drag = { uid: hit.uid, dx: hit.x - x, dy: hit.y - y, moved: false };
    drawLayoutCanvas();
  });
  cv.addEventListener("mousemove", function (e) {
    if (!drag) return;
    var r = cv.getBoundingClientRect();
    var x = (e.clientX - r.left) / S.zoom, y = (e.clientY - r.top) / S.zoom;
    var inst = instanceByUid(L, drag.uid);
    if (!inst) return;
    inst.x = Math.round(x + drag.dx);
    inst.y = Math.round(y + drag.dy);
    drag.moved = true;
    drawLayoutCanvas();
    buildProps();
  });
  window.addEventListener("mouseup", function () {
    if (drag && drag.moved) status("D\u00E9plac\u00E9 \u2192 " + drag.uid);
    drag = null;
  });
  cv.addEventListener("contextmenu", function (e) { e.preventDefault(); });
  drawLayoutCanvas();
}
function instanceByUid(L, uid) {
  var out = null;
  (L.layers || []).forEach(function (ly) {
    (ly.instances || []).forEach(function (i) { if (i.uid === uid) out = i; });
  });
  return out;
}
function pickInstance(L, x, y) {
  var found = null;
  for (var li = L.layers.length - 1; li >= 0 && !found; li--) {
    var ly = L.layers[li];
    if (ly.visible === false) continue;
    var insts = (ly.instances || []).slice().sort(function (a, b) { return (b.z || 0) - (a.z || 0); });
    for (var q = 0; q < insts.length; q++) {
      var i = insts[q];
      if (x >= i.x && x <= i.x + i.width && y >= i.y && y <= i.y + i.height) { found = i; break; }
    }
  }
  return found;
}
function drawLayoutCanvas() {
  var cv = document.getElementById("layoutcanvas");
  if (!cv) return;
  var L = findLayout(cv.dataset.layout);
  if (!L) return;
  var ctx = cv.getContext("2d");
  var z = S.zoom;
  ctx.setTransform(z, 0, 0, z, 0, 0);
  ctx.fillStyle = L.background || "#ffffff";
  ctx.fillRect(0, 0, L.width, L.height);
  // grille
  ctx.strokeStyle = "rgba(0,0,0,0.07)";
  ctx.lineWidth = 1;
  for (var gx = 0; gx <= L.width; gx += 50) { ctx.beginPath(); ctx.moveTo(gx, 0); ctx.lineTo(gx, L.height); ctx.stroke(); }
  for (var gy = 0; gy <= L.height; gy += 50) { ctx.beginPath(); ctx.moveTo(0, gy); ctx.lineTo(L.width, gy); ctx.stroke(); }
  (L.layers || []).forEach(function (ly, li) {
    if (ly.visible === false) return;
    ctx.globalAlpha = Math.max(0, Math.min(1, (ly.opacity == null ? 100 : ly.opacity) / 100));
    (ly.instances || []).slice().sort(function (a, b) { return (a.z || 0) - (b.z || 0); }).forEach(function (i) {
      drawInstance(ctx, i);
      if (S.sel && S.sel.kind === "instance" && S.sel.uid === i.uid) {
        ctx.strokeStyle = "#3aa0ff"; ctx.lineWidth = 2 / z;
        ctx.strokeRect(i.x - 1, i.y - 1, i.width + 2, i.height + 2);
      }
    });
  });
  ctx.globalAlpha = 1;
}
function drawInstance(ctx, i) {
  var t = findType(i.object);
  if (!t) {
    ctx.fillStyle = "#c05050";
    ctx.fillRect(i.x, i.y, i.width, i.height);
    return;
  }
  if (t.plugin === "tiledbg") {
    var im = assetImage(t.texture || (t.properties && t.properties.texture));
    if (im && im.width) {
      var pat = ctx.createPattern(im, "repeat");
      ctx.save();
      ctx.fillStyle = pat;
      ctx.fillRect(i.x, i.y, i.width, i.height);
      ctx.restore();
    } else {
      ctx.fillStyle = "#9a9a9a";
      ctx.fillRect(i.x, i.y, i.width, i.height);
    }
    return;
  }
  if (t.plugin === "text") {
    ctx.save();
    ctx.fillStyle = (i.properties && i.properties.color) || (t.properties && t.properties.color) || "#111";
    var size = Number((i.properties && i.properties.size) || (t.properties && t.properties.size) || 18);
    ctx.font = size + "px Arial";
    ctx.textBaseline = "top";
    var txt = String((i.properties && i.properties.text) || (t.properties && t.properties.text) || "Texte");
    String(txt).split("\n").forEach(function (ln, k) { ctx.fillText(ln, i.x, i.y + k * size * 1.25); });
    ctx.restore();
    return;
  }
  var fid = "";
  if (t.animations && t.animations.length && t.animations[0].frames && t.animations[0].frames.length) fid = t.animations[0].frames[0].texture;
  var im2 = fid ? assetImage(fid) : null;
  if (im2 && im2.width) {
    ctx.save();
    ctx.translate(i.x + i.width / 2, i.y + i.height / 2);
    ctx.rotate((i.angle || 0) * Math.PI / 180);
    ctx.drawImage(im2, -i.width / 2, -i.height / 2, i.width, i.height);
    ctx.restore();
  } else {
    ctx.fillStyle = "#b8bcc4";
    ctx.fillRect(i.x, i.y, i.width, i.height);
    ctx.strokeStyle = "#7a7f88";
    ctx.strokeRect(i.x, i.y, i.width, i.height);
  }
}
function zoomIn() { S.zoom = Math.min(4, S.zoom * 1.25); rebuildCanvas(); }
function zoomOut() { S.zoom = Math.max(0.1, S.zoom / 1.25); rebuildCanvas(); }
function zoomReset() { S.zoom = 1; rebuildCanvas(); }
function rebuildCanvas() {
  var cv = document.getElementById("layoutcanvas");
  if (!cv) return;
  var L = findLayout(cv.dataset.layout);
  if (!L) return;
  cv.width = Math.floor(L.width * S.zoom);
  cv.height = Math.floor(L.height * S.zoom);
  drawLayoutCanvas();
}

/* ---- event sheet view ---- */
function buildSheetView(v, sname) {
  var sh = findSheet(sname);
  if (!sh) return;
  var tb = el("div", "sheet-toolbar");
  var addEv = el("button", "primary", "+ Ajouter un \u00E9v\u00E9nement");
  addEv.addEventListener("click", function () {
    sh.events.push({ disabled: false, is_group: false, group_name: "", conditions: [], actions: [], children: [] });
    saveProject().then(function () { buildView(); buildTree(); });
  });
  tb.appendChild(addEv);
  var addVar = el("button", null, "+ Variable globale");
  addVar.addEventListener("click", addGlobalVar);
  tb.appendChild(addVar);
  v.appendChild(tb);

  var w = el("div", "sheet");
  if (!sh.events.length) w.appendChild(el("div", "ev-empty", "Aucun \u00E9v\u00E9nement. Ajoutez-en un : condition(s) \u2192 action(s)."));
  sh.events.forEach(function (ev, ei) {
    w.appendChild(eventRow(sh, ei, ev, false));
  });
  v.appendChild(w);
}
function eventRow(sh, ei, ev, isChild) {
  var row = el("div", "ev-row");
  var num = el("div", "ev-num", String(ei + 1));
  row.appendChild(num);
  var body = el("div", "ev-body");
  if (ev.is_group && ev.group_name) {
    var gh = el("div", "cond-line");
    gh.appendChild(el("span", "obj", "\u25BC Groupe : " + ev.group_name));
    body.appendChild(gh);
  }
  var conds = el("div", "ev-conds");
  if (!ev.conditions.length) conds.appendChild(el("div", "ev-empty", "(aucune condition \u2192 toujours vrai)"));
  (ev.conditions || []).forEach(function (c, ci) {
    var def = condDef(c.id);
    var line = el("div", "cond-line");
    line.appendChild(el("span", "obj", c.object === "system" ? "Syst\u00E8me" : c.object));
    line.appendChild(el("span", null, def.label + (def.params.length ? "  \u2192  " + fmtParams(c.params) : "")));
    line.style.cursor = "pointer";
    line.addEventListener("click", function () {
      modalParams("\u00C9diter la condition", def, c.params, function (vals) {
        c.params = vals;
        saveProject().then(function () { buildView(); });
      }, function () {
        ev.conditions.splice(ci, 1);
        saveProject().then(function () { buildView(); });
      });
    });
    conds.appendChild(line);
  });
  body.appendChild(conds);
  var acts = el("div", "ev-acts");
  if (!ev.actions.length) acts.appendChild(el("div", "ev-empty", "(aucune action)"));
  (ev.actions || []).forEach(function (a, ai) {
    var def = actDef(a.id);
    var line = el("div", "act-line");
    line.appendChild(el("span", "obj", a.object === "system" ? "Syst\u00E8me" : a.object));
    line.appendChild(el("span", null, def.label + (def.params.length ? "  \u2192  " + fmtParams(a.params) : "")));
    line.style.cursor = "pointer";
    line.addEventListener("click", function () {
      modalParams("\u00C9diter l'action", def, a.params, function (vals) {
        a.params = vals;
        saveProject().then(function () { buildView(); });
      }, function () {
        ev.actions.splice(ai, 1);
        saveProject().then(function () { buildView(); });
      });
    });
    acts.appendChild(line);
  });
  body.appendChild(acts);
  var tools = el("div", "ev-tools");
  var bc = el("button", null, "+ condition");
  bc.addEventListener("click", function () { pickCondDialog(function (cond) { ev.conditions.push(cond); saveProject().then(function () { buildView(); }); }); });
  var ba = el("button", null, "+ action");
  ba.addEventListener("click", function () { pickActDialog(function (act) { ev.actions.push(act); saveProject().then(function () { buildView(); }); }); });
  var bx = el("button", null, "\u00D7 supprimer");
  bx.addEventListener("click", function () { sh.events.splice(ei, 1); saveProject().then(function () { buildView(); buildTree(); }); });
  tools.appendChild(bc); tools.appendChild(ba); tools.appendChild(bx);
  body.appendChild(tools);
  row.appendChild(body);
  if (ev.children && ev.children.length) {
    var sub = el("div", "sub-events");
    ev.children.forEach(function (c2, ci2) { sub.appendChild(eventRow(sh, ci2, c2, true)); });
    row.appendChild(sub);
  }
  return row;
}

/* ------------------------------------------------------------ dialogs */

function modal(title, bodyEl, footBtns) {
  var back = el("div", "modal-back");
  var m = el("div", "modal");
  m.appendChild(el("h3", null, title));
  var body = el("div", "modal-body");
  body.appendChild(bodyEl);
  m.appendChild(body);
  var foot = el("div", "modal-foot");
  footBtns.forEach(function (b) {
    var btn = el("button", b[1] ? "primary" : "", b[0]);
    btn.addEventListener("click", b[2]);
    foot.appendChild(btn);
  });
  m.appendChild(foot);
  back.appendChild(m);
  $("#modal-root").appendChild(back);
  back.addEventListener("click", function (e) { if (e.target === back) back.remove(); });
  return back;
}
function fileInput(accept, multiple, onFiles) {
  var inp = document.createElement("input");
  inp.type = "file";
  if (accept) inp.accept = accept;
  inp.multiple = !!multiple;
  inp.addEventListener("change", function () {
    if (inp.files && inp.files.length) onFiles(Array.prototype.slice.call(inp.files));
  });
  inp.style.display = "none";
  document.body.appendChild(inp);
  inp.click();
  setTimeout(function () { inp.remove(); }, 60000);
}
function openC3pDialog() { fileInput(".c3p,.capx,.zip", false, function (files) { importFiles(files); }); }
function importDialog() { fileInput(null, true, function (files) { importFiles(files); }); }
async function importFiles(files) {
  for (var i = 0; i < files.length; i++) {
    var f = files[i];
    var buf = await f.arrayBuffer();
    var low = f.name.toLowerCase();
    if (low.endsWith(".c3p") || low.endsWith(".capx")) {
      await openC3pBytes(buf, f.name);
    } else {
      var r = await api("POST", "/api/import?name=" + encodeURIComponent(f.name), buf);
      if (r && r.ok) {
        S.project = r.project;
        (r.warnings || []).forEach(function (w) { toast(w, "warn"); });
        var ids = r.ids || [];
        var a = ids.length ? assetById(ids[0]) : null;
        toast("Import\u00E9 : " + f.name + (a ? "  [" + a.kind + "]" : ""));
      } else {
        toast("\u00C9chec import : " + f.name, "err");
      }
    }
  }
  await saveProject();
  buildTree(); buildProps(); buildTabs(); buildView();
}
async function openC3pBytes(buf, fname) {
  var r = await api("POST", "/api/open", buf);
  if (r && r.ok) {
    S.project = r.project;
    S.warnings = r.warnings || [];
    S.openTabs = ["start"];
    S.tab = "start";
    S.sel = null;
    toast("Projet charg\u00E9 : " + S.project.name + (fname ? " (" + fname + ")" : ""));
    S.warnings.forEach(function (w) { toast(w, "warn"); });
    buildTabs(); buildView(); buildTree(); buildProps();
  } else {
    toast("Erreur : " + ((r && r.error) || "fichier illisible"), "err");
  }
}
function saveC3p() {
  saveProject().then(function () {
    return fetch("/api/save", { method: "POST" });
  }).then(function (r) {
    if (!r.ok) throw new Error("HTTP " + r.status);
    return r.blob();
  }).then(function (b) {
    downloadBlob(b, (S.project.name || "projet") + ".c3p");
    toast("Projet enregistr\u00E9 en .c3p");
  }).catch(function (e) { toast("Erreur : " + e.message, "err"); });
}
function exportHtml() {
  saveProject().then(function () {
    return fetch("/api/export", { method: "POST" });
  }).then(function (r) {
    if (!r.ok) throw new Error("HTTP " + r.status);
    return r.blob();
  }).then(function (b) {
    downloadBlob(b, (S.project.name || "jeu") + ".html");
    toast("Export HTML5 autonome g\u00E9n\u00E9r\u00E9");
  }).catch(function (e) { toast("Erreur : " + e.message, "err"); });
}
function downloadBlob(blob, name) {
  var a = document.createElement("a");
  a.href = URL.createObjectURL(blob);
  a.download = name;
  document.body.appendChild(a);
  a.click();
  setTimeout(function () { URL.revokeObjectURL(a.href); a.remove(); }, 4000);
}

/* ---- picker dialogs ---- */
function pickDialog(title, defs, getObjLabel, onPick) {
  var list = el("div", "pick-list");
  var lastCat = null;
  defs.forEach(function (d) {
    if (d.cat !== lastCat) { list.appendChild(el("div", "pick-cat", d.cat)); lastCat = d.cat; }
    var it = el("div", "pick-item", d.label);
    it.addEventListener("click", function () {
      back.remove();
      if (d.obj) {
        pickObjectDialog(function (objName) {
          modalParams(title + " : " + d.label, d, [], function (vals) {
            onPick({ id: d.id, object: objName, params: vals });
          }, null);
        });
      } else {
        modalParams(title + " : " + d.label, d, [], function (vals) {
          onPick({ id: d.id, object: "system", params: vals });
        }, null);
      }
    });
    list.appendChild(it);
  });
  var back = modal(title, list, [["Annuler", false, function () { back.remove(); }]]);
}
function pickCondDialog(cb) { pickDialog("Ajouter une condition", CONDS, null, cb); }
function pickActDialog(cb) { pickDialog("Ajouter une action", ACTIONS, null, cb); }
function pickObjectDialog(cb) {
  var list = el("div", "pick-list");
  S.project.object_types.forEach(function (t) {
    if (t.plugin === "system") return;
    var it = el("div", "pick-item", t.name + "  (" + t.plugin + ")");
    it.addEventListener("click", function () { back.remove(); cb(t.name); });
    list.appendChild(it);
  });
  var back = modal("Choisir un objet", list, [["Annuler", false, function () { back.remove(); }]]);
}

/* ---- params form ---- */
function paramInput(def, existing) {
  var val = existing;
  var wrap = el("div", "prop-row");
  wrap.appendChild(el("label", null, def.k));
  var inp;
  if (def.t === "select") {
    inp = el("select");
    def.opts.forEach(function (o) {
      var op = el("option", null, o); op.value = o;
      inp.appendChild(op);
    });
    inp.value = val == null ? def.opts[0] : String(val);
  } else if (def.t === "key") {
    inp = el("select");
    KEYS.forEach(function (o) {
      var op = el("option", null, o); op.value = o;
      inp.appendChild(op);
    });
    inp.value = val == null ? "space" : String(val).toLowerCase();
  } else if (def.t === "object") {
    inp = el("select");
    S.project.object_types.forEach(function (t) {
      if (t.plugin === "system") return;
      var op = el("option", null, t.name); op.value = t.name;
      inp.appendChild(op);
    });
    if (val != null) inp.value = String(val);
  } else if (def.t === "layer") {
    inp = el("select");
    var cur = findLayout(currentLayoutName());
    (cur ? cur.layers : (S.project.layouts[0] || { layers: [] }).layers).forEach(function (ly) {
      var op = el("option", null, ly.name); op.value = ly.name;
      inp.appendChild(op);
    });
    if (val != null) inp.value = String(val);
  } else if (def.t === "layout") {
    inp = el("select");
    S.project.layouts.forEach(function (l) {
      var op = el("option", null, l.name); op.value = l.name;
      inp.appendChild(op);
    });
    if (val != null) inp.value = String(val);
  } else if (def.t === "gvar") {
    inp = el("select");
    S.project.global_vars.forEach(function (g) {
      var op = el("option", null, g.name); op.value = g.name;
      inp.appendChild(op);
    });
    var newOp = el("option", null, "\u2795 nouvelle variable\u2026"); newOp.value = "__new__";
    inp.appendChild(newOp);
    inp.addEventListener("change", function () {
      if (inp.value === "__new__") {
        var nm = prompt("Nom de la variable globale :");
        if (nm) {
          S.project.global_vars.push({ name: nm, type: "number", initial: 0 });
          var op = el("option", null, nm); op.value = nm;
          inp.insertBefore(op, newOp);
          inp.value = nm;
        } else inp.value = inp.options[0] ? inp.options[0].value : "";
      }
    });
    if (val != null) inp.value = String(val);
  } else if (def.t === "asset-audio") {
    inp = el("select");
    var auds = S.project.assets.filter(function (a) { return a.kind === "audio"; });
    if (!auds.length) { var op0 = el("option", null, "(aucun son \u2014 importez un .wav/.ogg/.mp3)"); op0.value = ""; inp.appendChild(op0); }
    auds.forEach(function (a) {
      var op = el("option", null, a.name); op.value = a.name;
      inp.appendChild(op);
    });
    if (val != null) inp.value = String(val);
  } else if (def.t === "number") {
    inp = el("input");
    inp.type = "number";
    inp.step = "any";
    inp.value = val == null ? "0" : String(val);
  } else {
    inp = el("input");
    inp.type = "text";
    inp.value = val == null ? "" : String(val);
  }
  wrap.appendChild(inp);
  return { row: wrap, input: inp };
}
function currentLayoutName() {
  if (S.tab.indexOf("layout:") === 0) return S.tab.slice(7);
  return (S.project.layouts[0] || { name: "" }).name;
}
function modalParams(title, def, existing, onSubmit, onDelete) {
  var form = el("div");
  var inputs = [];
  def.params.forEach(function (pd, i) {
    var r = paramInput(pd, existing[i]);
    form.appendChild(r.row);
    inputs.push(r.input);
  });
  if (!def.params.length) form.appendChild(el("div", "ev-empty", "Aucun param\u00E8tre."));
  var foot = [["Annuler", false, function () { back.remove(); }]];
  if (onDelete) foot.unshift(["Supprimer", false, function () { back.remove(); onDelete(); }]);
  foot.push(["OK", true, function () {
    var vals = inputs.map(function (inp) {
      if (inp.tagName === "SELECT") return inp.value;
      if (inp.type === "number") return inp.value === "" ? 0 : Number(inp.value);
      return inp.value;
    });
    back.remove();
    onSubmit(vals);
  }]);
  var back = modal(title, form, foot);
}

/* ------------------------------------------------------------ props panel */

function buildProps() {
  var p = $("#props");
  p.innerHTML = "";
  if (!S.sel) { projectProps(p); return; }
  if (S.sel.kind === "instance") instanceProps(p);
  else if (S.sel.kind === "type") typeProps(p);
  else if (S.sel.kind === "asset") assetProps(p);
  else projectProps(p);
}
function propRow(parent, label, value, onChange, opts) {
  var row = el("div", "prop-row");
  row.appendChild(el("label", null, label));
  var inp;
  if (opts && opts.select) {
    inp = el("select");
    opts.select.forEach(function (o) {
      var op = el("option", null, o); op.value = o;
      inp.appendChild(op);
    });
    inp.value = String(value);
  } else {
    inp = el("input");
    inp.type = opts && opts.number ? "number" : "text";
    if (opts && opts.number) inp.step = "any";
    inp.value = value == null ? "" : String(value);
  }
  inp.addEventListener("change", function () {
    var v = (opts && opts.number) ? (inp.value === "" ? 0 : Number(inp.value)) : inp.value;
    onChange(v);
  });
  row.appendChild(inp);
  parent.appendChild(row);
  return inp;
}
function projectProps(p) {
  var sec = el("div", "props-section");
  sec.appendChild(el("div", "props-title", "Propri\u00E9t\u00E9s du projet"));
  propRow(sec, "Nom", S.project.name, function (v) { S.project.name = v; buildTree(); });
  propRow(sec, "Auteur", S.project.author, function (v) { S.project.author = v; });
  propRow(sec, "Largeur", S.project.width, function (v) { S.project.width = v; }, { number: true });
  propRow(sec, "Hauteur", S.project.height, function (v) { S.project.height = v; }, { number: true });
  var sel = el("div", "prop-row");
  sel.appendChild(el("label", null, "Layout initial"));
  var sl = el("select");
  S.project.layouts.forEach(function (l) {
    var op = el("option", null, l.name); op.value = l.name;
    sl.appendChild(op);
  });
  sl.value = S.project.first_layout;
  sl.addEventListener("change", function () { S.project.first_layout = sl.value; });
  sel.appendChild(sl);
  sec.appendChild(sel);
  p.appendChild(sec);
  var sec2 = el("div", "props-section");
  sec2.appendChild(el("div", "props-title", "Variables globales"));
  if (!S.project.global_vars.length) sec2.appendChild(el("div", "ev-empty", "Aucune."));
  S.project.global_vars.forEach(function (g, gi) {
    var row = el("div", "prop-row");
    row.appendChild(el("label", null, g.name));
    var inp = el("input");
    inp.type = "text";
    inp.value = String(g.initial);
    inp.addEventListener("change", function () { g.initial = inp.value; });
    row.appendChild(inp);
    var del = el("div", "add", "\u00D7");
    del.title = "Supprimer";
    del.addEventListener("click", function () { S.project.global_vars.splice(gi, 1); buildProps(); });
    row.appendChild(del);
    sec2.appendChild(row);
  });
  var addv = el("button", null, "+ Variable");
  addv.addEventListener("click", addGlobalVar);
  sec2.appendChild(addv);
  p.appendChild(sec2);
}
function instanceProps(p) {
  var L = findLayout(S.sel.layout || currentLayoutName());
  var inst = L ? instanceByUid(L, S.sel.uid) : null;
  if (!inst) { S.sel = null; projectProps(p); return; }
  var sec = el("div", "props-section");
  sec.appendChild(el("div", "props-title", inst.object + "  #inst"));
  var upd = function () { drawLayoutCanvas(); };
  propRow(sec, "X", Math.round(inst.x), function (v) { inst.x = v; upd(); }, { number: true });
  propRow(sec, "Y", Math.round(inst.y), function (v) { inst.y = v; upd(); }, { number: true });
  propRow(sec, "Largeur", Math.round(inst.width), function (v) { inst.width = v; upd(); }, { number: true });
  propRow(sec, "Hauteur", Math.round(inst.height), function (v) { inst.height = v; upd(); }, { number: true });
  propRow(sec, "Angle", inst.angle, function (v) { inst.angle = v; upd(); }, { number: true });
  propRow(sec, "Opacit\u00E9", inst.opacity, function (v) { inst.opacity = v; upd(); }, { number: true });
  propRow(sec, "Z order", inst.z, function (v) { inst.z = v; upd(); }, { number: true });
  if (inst.object && inst.object.indexOf("Text") >= 0 || (inst.properties && inst.properties.text !== undefined)) {
    propRow(sec, "Texte", inst.properties.text || "", function (v) { inst.properties.text = v; upd(); });
    propRow(sec, "Taille", inst.properties.size || 18, function (v) { inst.properties.size = v; upd(); }, { number: true });
    propRow(sec, "Couleur", inst.properties.color || "#000000", function (v) { inst.properties.color = v; upd(); });
  }
  var t = findType(inst.object);
  if (t && t.instance_vars.length) {
    sec.appendChild(el("div", "props-title", "Variables d'instance"));
    t.instance_vars.forEach(function (v) {
      propRow(sec, v.name, inst.vars[v.name] == null ? "" : inst.vars[v.name], function (val) { inst.vars[v.name] = val; }, { number: v.type === "number" });
    });
  }
  var del = el("button", null, "\uD83D\uDDD1 Supprimer l'instance");
  del.style.width = "100%";
  del.addEventListener("click", function () {
    (L.layers || []).forEach(function (ly) {
      ly.instances = (ly.instances || []).filter(function (x) { return x.uid !== inst.uid; });
    });
    S.sel = null;
    buildProps(); drawLayoutCanvas(); buildTree();
  });
  sec.appendChild(del);
  p.appendChild(sec);
}
function typeProps(p) {
  var t = findType(S.sel.name);
  if (!t) { S.sel = null; projectProps(p); return; }
  var sec = el("div", "props-section");
  sec.appendChild(el("div", "props-title", "Objet : " + t.name));
  propRow(sec, "Nom", t.name, function (v) {
    var old = t.name; t.name = v;
    S.project.layouts.forEach(function (L) {
      (L.layers || []).forEach(function (ly) {
        (ly.instances || []).forEach(function (i) { if (i.object === old) i.object = v; });
      });
    });
    S.project.event_sheets.forEach(function (sh) {
      walkEvents(sh.events, function (e) {
        (e.conditions || []).forEach(function (c) { if (c.object === old) c.object = v; });
        (e.actions || []).forEach(function (a) { if (a.object === old) a.object = v; });
      });
    });
    S.sel = { kind: "type", name: v };
    buildTree(); buildProps();
  });
  propRow(sec, "Plugin", t.plugin, function () {}, {});
  var beh = el("div", "ev-empty", t.behaviors.length ? "Behaviors : " + t.behaviors.map(function (b) { return b.id; }).join(", ") : "Aucun behavior");
  sec.appendChild(beh);
  var addL = el("button", null, "+ Ajouter au layout courant");
  addL.style.width = "100%";
  addL.addEventListener("click", function () { addInstanceToLayout(t.name); });
  sec.appendChild(addL);
  var delT = el("button", null, "\uD83D\uDDD1 Supprimer l'objet");
  delT.style.width = "100%";
  delT.style.marginTop = "6px";
  delT.addEventListener("click", function () {
    S.project.object_types = S.project.object_types.filter(function (x) { return x.name !== t.name; });
    S.project.layouts.forEach(function (L) {
      (L.layers || []).forEach(function (ly) {
        ly.instances = (ly.instances || []).filter(function (i) { return i.object !== t.name; });
      });
    });
    S.sel = null;
    buildTree(); buildProps();
  });
  sec.appendChild(delT);
  p.appendChild(sec);
}
function walkEvents(events, f) {
  (events || []).forEach(function (e) {
    f(e);
    walkEvents(e.children, f);
  });
}
function assetProps(p) {
  var a = assetById(S.sel.id);
  if (!a) { S.sel = null; projectProps(p); return; }
  var sec = el("div", "props-section");
  sec.appendChild(el("div", "props-title", "Ressource"));
  if (a.kind === "image") {
    var im = el("img", "asset-preview");
    im.src = a.data;
    sec.appendChild(im);
  } else if (a.kind === "audio") {
    var au = el("audio");
    au.src = a.data;
    au.controls = true;
    au.style.width = "100%";
    sec.appendChild(au);
  } else if (a.kind === "video") {
    var vd = el("video");
    vd.src = a.data;
    vd.controls = true;
    vd.style.width = "100%";
    sec.appendChild(vd);
  }
  propRow(sec, "Nom", a.name, function (v) { a.name = v; buildTree(); });
  var info = el("div", "ev-empty", a.kind + " \u00B7 " + a.mime + (a.width ? " \u00B7 " + a.width + "\u00D7" + a.height : ""));
  sec.appendChild(info);
  if (a.kind === "image") {
    var mk = el("button", null, "\u2192 Cr\u00E9er un Sprite avec cette image");
    mk.style.width = "100%";
    mk.addEventListener("click", function () { spriteFromAsset(a); });
    sec.appendChild(mk);
  }
  var dl = el("button", null, "\u2193 T\u00E9l\u00E9charger");
  dl.style.width = "100%";
  dl.style.marginTop = "6px";
  dl.addEventListener("click", function () {
    fetch("/api/asset/" + a.id).then(function (r) { return r.blob(); }).then(function (b) { downloadBlob(b, a.name); });
  });
  sec.appendChild(dl);
  p.appendChild(sec);
}

/* ------------------------------------------------------------ creation */

function addInstanceToLayout(typeName, x, y) {
  var L = findLayout(currentLayoutName());
  if (!L) { toast("Aucun layout", "err"); return; }
  if (!L.layers.length) L.layers.push({ name: "Layer 0", visible: true, opacity: 100, parallax_x: 100, parallax_y: 100, instances: [] });
  var uid = 1;
  L.layers.forEach(function (ly) { (ly.instances || []).forEach(function (i) { if (i.uid >= uid) uid = i.uid + 1; }); });
  var t = findType(typeName);
  var inst = {
    uid: uid, object: typeName,
    x: x == null ? 120 : x, y: y == null ? 120 : y,
    width: 32, height: 32, angle: 0, z: 0, opacity: 100,
    vars: {}, properties: {}
  };
  if (t) {
    var fid = t.texture || (t.properties && t.properties.texture);
    if (!fid && t.animations && t.animations[0] && t.animations[0].frames && t.animations[0].frames[0]) fid = t.animations[0].frames[0].texture;
    var im = fid ? assetImage(fid) : null;
    if (im && im.width) { inst.width = im.width; inst.height = im.height; }
    (t.instance_vars || []).forEach(function (v) { inst.vars[v.name] = v.initial; });
    if (t.plugin === "text") {
      inst.properties.text = t.properties.text || "Texte";
      inst.properties.size = t.properties.size || 18;
      inst.properties.color = t.properties.color || "#000000";
      inst.width = 200; inst.height = 30;
    }
  }
  L.layers[0].instances.push(inst);
  S.sel = { kind: "instance", layout: L.name, uid: uid };
  saveProject().then(function () {
    buildTree(); buildProps();
    if (S.tab !== "layout:" + L.name) openTab("layout:" + L.name); else { buildView(); }
  });
}
function spriteFromAsset(a) {
  var name = a.name.replace(/\.[a-z0-9]+$/i, "").replace(/[^a-zA-Z0-9_]/g, "_") || "Sprite";
  var base = name, k = 1;
  while (findType(name)) { k++; name = base + k; }
  S.project.object_types.push({
    uid: nextUid(), name: name, plugin: "sprite", is_global: false,
    behaviors: [], instance_vars: [],
    animations: [{ name: "Default", speed: 5, looping: true, frames: [{ texture: a.id, duration: 1 }] }],
    properties: {}, texture: ""
  });
  toast("Sprite \u00AB " + name + " \u00BB cr\u00E9\u00E9 \u2014 ajout\u00E9 au layout.");
  saveProject().then(function () {
    addInstanceToLayout(name);
    buildTree();
  });
}
function addObjectDialog() {
  var list = el("div", "pick-list");
  [
    ["Sprite", addSprite], ["Tiled Background", addTiled], ["Texte", addText],
    ["Clavier (global)", addKeyboard], ["Souris (global)", addMouse], ["Audio (global)", addAudio]
  ].forEach(function (o) {
    var it = el("div", "pick-item", o[0]);
    it.addEventListener("click", function () { back.remove(); o[1](); });
    list.appendChild(it);
  });
  var back = modal("Ajouter un objet", list, [["Annuler", false, function () { back.remove(); }]]);
}
function uniqueTypeName(base) {
  var name = base, k = 1;
  while (findType(name)) { k++; name = base + k; }
  return name;
}
function addSprite() { var nm = uniqueTypeName("Sprite"); S.project.object_types.push({ uid: nextUid(), name: nm, plugin: "sprite", is_global: false, behaviors: [], instance_vars: [], animations: [{ name: "Default", speed: 5, looping: true, frames: [] }], properties: {}, texture: "" }); saveProject().then(function () { S.sel = { kind: "type", name: nm }; buildTree(); buildProps(); toast("Sprite ajout\u00E9 \u2014 importez une image puis \u00AB Cr\u00E9er un Sprite \u00BB, ou ajoutez-le au layout."); }); }
function addTiled() { var nm = uniqueTypeName("TiledBg"); S.project.object_types.push({ uid: nextUid(), name: nm, plugin: "tiledbg", is_global: false, behaviors: [], instance_vars: [], animations: [], properties: {}, texture: "" }); saveProject().then(function () { S.sel = { kind: "type", name: nm }; buildTree(); buildProps(); }); }
function addText() { var nm = uniqueTypeName("Text"); S.project.object_types.push({ uid: nextUid(), name: nm, plugin: "text", is_global: false, behaviors: [], instance_vars: [], animations: [], properties: { text: "Texte", size: 18, color: "#000000" }, texture: "" }); saveProject().then(function () { addInstanceToLayout(nm); buildTree(); }); }
function addKeyboard() { var nm = uniqueTypeName("Keyboard"); S.project.object_types.push({ uid: nextUid(), name: nm, plugin: "keyboard", is_global: true, behaviors: [], instance_vars: [], animations: [], properties: {}, texture: "" }); saveProject().then(function () { buildTree(); }); }
function addMouse() { var nm = uniqueTypeName("Mouse"); S.project.object_types.push({ uid: nextUid(), name: nm, plugin: "mouse", is_global: true, behaviors: [], instance_vars: [], animations: [], properties: {}, texture: "" }); saveProject().then(function () { buildTree(); }); }
function addAudio() { var nm = uniqueTypeName("Audio"); S.project.object_types.push({ uid: nextUid(), name: nm, plugin: "audio", is_global: true, behaviors: [], instance_vars: [], animations: [], properties: {}, texture: "" }); saveProject().then(function () { buildTree(); }); }
function addLayout() {
  var nm = uniqueTypeName2("Layout ", S.project.layouts.map(function (l) { return l.name; }));
  S.project.layouts.push({ uid: nextUid(), name: nm, width: S.project.width, height: S.project.height, background: "#ffffff", layers: [{ name: "Layer 0", visible: true, opacity: 100, parallax_x: 100, parallax_y: 100, instances: [] }], event_sheet: (S.project.event_sheets[0] || { name: "" }).name });
  saveProject().then(function () { buildTree(); openTab("layout:" + nm); });
}
function addSheet() {
  var nm = uniqueTypeName2("Event sheet ", S.project.event_sheets.map(function (s) { return s.name; }));
  S.project.event_sheets.push({ uid: nextUid(), name: nm, events: [] });
  saveProject().then(function () { buildTree(); openTab("sheet:" + nm); });
}
function uniqueTypeName2(prefix, existing) {
  var k = 1;
  while (existing.indexOf(prefix + k) >= 0) k++;
  return prefix + k;
}
function addGlobalVar() {
  var nm = prompt("Nom de la variable globale :");
  if (!nm) return;
  S.project.global_vars.push({ name: nm, type: "number", initial: 0 });
  saveProject().then(function () { buildProps(); });
}
async function newProject() {
  var r = await api("POST", "/api/new");
  S.project = r.project;
  S.openTabs = ["start"]; S.tab = "start"; S.sel = null;
  buildTabs(); buildView(); buildTree(); buildProps();
  toast("Nouveau projet cr\u00E9\u00E9");
}

/* ------------------------------------------------------------ play */

function play() {
  saveProject().then(function () {
    var proj = JSON.stringify(S.project).replace(/</g, "\\u003c");
    var html = "<!DOCTYPE html><html><head><meta charset='utf-8'><style>html,body{margin:0;height:100%;background:#111;overflow:hidden}</style></head><body><scr" + "ipt>";
    html += S.playerSrc.replace(/<\/script/gi, "<\\/script");
    html += "\nwindow.C3R_PROJECT=" + proj + ";\nnew C3R_Runtime(window.C3R_PROJECT).start();\n</scr" + "ipt></body></html>";
    var ov = el("div", "play-overlay");
    var top = el("div", "play-top");
    var stop = el("button", "primary", "\u25A0 Arr\u00EAter (F5)");
    stop.addEventListener("click", function () { ov.remove(); });
    top.appendChild(stop);
    ov.appendChild(top);
    var fr = el("iframe");
    fr.srcdoc = html;
    ov.appendChild(fr);
    document.body.appendChild(ov);
    ov.dataset.playing = "1";
  });
}
function playVisible() { return document.querySelector(".play-overlay"); }

/* ------------------------------------------------------------ about */

function about() {
  var d = el("div");
  d.appendChild(el("p", null, "C3R \u2014 Construct 3 Rust Edition v0.1"));
  d.appendChild(el("p", null, "R\u00E9impl\u00E9mentation open source ind\u00E9pendante inspir\u00E9e de Construct 3 (Scirra). Aucune portion de code original de Scirra n'est utilis\u00E9e."));
  d.appendChild(el("p", null, "Importez n'importe quel fichier par glisser-d\u00E9poser : .c3p, images, sons, vid\u00E9os, polices, zip, JSON\u2026"));
  modal("\u00C0 propos", d, [["Fermer", true, function () { back.remove(); }]]);
  var back = document.querySelector(".modal-back:last-child");
}

/* ------------------------------------------------------------ dnd & hotkeys */

function setupDnd() {
  var hint = $("#drophint");
  var depth = 0;
  document.addEventListener("dragenter", function (e) { e.preventDefault(); depth++; hint.classList.add("on"); });
  document.addEventListener("dragover", function (e) { e.preventDefault(); });
  document.addEventListener("dragleave", function (e) { e.preventDefault(); depth--; if (depth <= 0) { depth = 0; hint.classList.remove("on"); } });
  document.addEventListener("drop", function (e) {
    e.preventDefault();
    depth = 0;
    hint.classList.remove("on");
    if (e.dataTransfer && e.dataTransfer.files && e.dataTransfer.files.length) {
      importFiles(Array.prototype.slice.call(e.dataTransfer.files));
    }
  });
}
function setupHotkeys() {
  document.addEventListener("keydown", function (e) {
    if (e.key === "F5") {
      e.preventDefault();
      if (playVisible()) { var ov = playVisible(); ov.remove(); } else play();
      return;
    }
    if (playVisible()) return;
    if (e.ctrlKey || e.metaKey) {
      var k = e.key.toLowerCase();
      if (k === "n") { e.preventDefault(); newProject(); }
      else if (k === "o") { e.preventDefault(); openC3pDialog(); }
      else if (k === "i") { e.preventDefault(); importDialog(); }
      else if (k === "s") { e.preventDefault(); saveC3p(); }
      else if (k === "e") { e.preventDefault(); exportHtml(); }
      return;
    }
    if ((e.key === "Delete" || e.key === "Backspace") && S.sel && S.sel.kind === "instance") {
      var L = findLayout(S.sel.layout || currentLayoutName());
      if (L) {
        L.layers.forEach(function (ly) {
          ly.instances = (ly.instances || []).filter(function (x) { return x.uid !== S.sel.uid; });
        });
        S.sel = null;
        buildProps(); drawLayoutCanvas(); buildTree();
        e.preventDefault();
      }
    }
    if (S.sel && S.sel.kind === "instance" && ["ArrowLeft", "ArrowRight", "ArrowUp", "ArrowDown"].indexOf(e.key) >= 0) {
      var L2 = findLayout(S.sel.layout || currentLayoutName());
      var inst = L2 ? instanceByUid(L2, S.sel.uid) : null;
      if (inst) {
        if (e.key === "ArrowLeft") inst.x -= 1;
        if (e.key === "ArrowRight") inst.x += 1;
        if (e.key === "ArrowUp") inst.y -= 1;
        if (e.key === "ArrowDown") inst.y += 1;
        buildProps(); drawLayoutCanvas();
        e.preventDefault();
      }
    }
  });
}

/* ------------------------------------------------------------ init */

function buildUI() {
  buildMenus();
  buildRibbon();
  buildTabs();
  buildView();
  buildTree();
  buildProps();
  setupDnd();
  setupHotkeys();
  $("#btn-play").addEventListener("click", play);
  status("Projet : " + S.project.name);
}
boot();
