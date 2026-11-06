/* C3R Runtime — moteur HTML5 original (inspiré des concepts de Construct 3, code 100% original) */
(function (global) {
"use strict";

function n(x, d) { var v = Number(x); return isFinite(v) ? v : (d || 0); }
function s(x, d) { return (x === undefined || x === null) ? (d || "") : String(x); }

var KEYMAP = {
  left: ["ArrowLeft"], right: ["ArrowRight"], up: ["ArrowUp"], down: ["ArrowDown"],
  space: [" "], enter: ["Enter"], shift: ["Shift"], control: ["Control"],
  w: ["w", "W"], a: ["a", "A"], s: ["s", "S"], d: ["d", "D"], e: ["e", "E"],
  q: ["q", "Q"], z: ["z", "Z"], x: ["x", "X"], c: ["c", "C"], v: ["v", "V"],
  n0: ["0"], n1: ["1"], n2: ["2"], n3: ["3"], n4: ["4"],
  n5: ["5"], n6: ["6"], n7: ["7"], n8: ["8"], n9: ["9"], esc: ["Escape"]
};
function normKey(k) {
  if (k === " " || k === "Spacebar") return "space";
  var m = { ArrowLeft: "left", ArrowRight: "right", ArrowUp: "up", ArrowDown: "down", Enter: "enter", Shift: "shift", Control: "control", Escape: "esc", Tab: "tab" };
  if (m[k]) return m[k];
  if (/^[a-z]$/i.test(k)) return k.toLowerCase();
  if (/^[0-9]$/.test(k)) return "n" + k;
  return k.toLowerCase();
}

function keyNames(v) {
  var out = [];
  function one(x) {
    var k = s(x).toLowerCase();
    if (KEYMAP[k]) { for (var i = 0; i < KEYMAP[k].length; i++) out.push(KEYMAP[k][i]); }
    else if (k) out.push(k);
  }
  if (Array.isArray(v)) v.forEach(one); else one(v);
  return out;
}

function aabb(a, b) {
  return a.x < b.x + b.width && a.x + a.width > b.x && a.y < b.y + b.height && a.y + a.height > b.y;
}
function pointIn(p, i) {
  return p.x >= i.x && p.x <= i.x + i.width && p.y >= i.y && p.y <= i.y + i.height;
}
function cmp(a, op, b) {
  a = n(a); b = n(b);
  switch (s(op, "==")) {
    case "==": case "equal": case "equal-to": case "egale": return a === b || Math.abs(a - b) < 1e-9;
    case "!=": case "not-equal": case "different": return Math.abs(a - b) >= 1e-9 && a !== b;
    case "<": case "less": case "inferieur": return a < b;
    case "<=": case "less-or-equal": return a <= b;
    case ">": case "greater": case "superieur": return a > b;
    case ">=": case "greater-or-equal": return a >= b;
  }
  return false;
}
function applyOp(a, op, b) {
  a = n(a); b = n(b);
  switch (s(op, "set")) {
    case "add": case "+": return a + b;
    case "sub": case "-": case "subtract": return a - b;
    case "mul": case "*": return a * b;
    case "div": case "/": return b !== 0 ? a / b : a;
    default: return b;
  }
}

function Behavior(def) {
  this.id = s(def.id);
  this.name = s(def.name, def.id);
  this.props = def.properties || {};
  this.state = {};
}
Behavior.prototype.num = function (keys, d) {
  for (var i = 0; i < keys.length; i++) {
    var v = this.props[keys[i]];
    if (v !== undefined && v !== "" && v !== null) return n(v, d);
  }
  return d;
};
Behavior.prototype.str = function (keys, d) {
  for (var i = 0; i < keys.length; i++) {
    var v = this.props[keys[i]];
    if (v !== undefined && v !== "") return String(v);
  }
  return d;
};

function Runtime(project) {
  var self = this;
  this.p = project || {};
  this.types = {};
  (this.p.object_types || []).forEach(function (t) { self.types[t.name] = t; });
  this.assets = {};
  (this.p.assets || []).forEach(function (a) {
    if (a.kind === "image" && a.data) {
      var im = new Image();
      im.src = a.data;
      self.assets[a.id] = im;
      self.assets["name:" + a.name] = im;
    }
  });
  this.sheets = {};
  (this.p.event_sheets || []).forEach(function (sh) { self.sheets[sh.name] = sh; });
  this.global = {};
  (this.p.global_vars || []).forEach(function (v) { self.global[v.name] = v.initial; });

  this.canvas = document.createElement("canvas");
  this.canvas.width = this.p.width || 1280;
  this.canvas.height = this.p.height || 720;
  document.body.style.cssText = "margin:0;height:100vh;display:flex;align-items:center;justify-content:center;background:#111;overflow:hidden;";
  this.canvas.style.cssText = "display:block;background:#000;max-width:100vw;max-height:100vh;";
  document.body.appendChild(this.canvas);
  this.ctx = this.canvas.getContext("2d");

  this.fit = function () {
    var r = Math.min(window.innerWidth / this.canvas.width, window.innerHeight / this.canvas.height);
    r = Math.max(0.1, Math.min(r, 2));
    this.canvas.style.width = Math.floor(this.canvas.width * r) + "px";
    this.canvas.style.height = Math.floor(this.canvas.height * r) + "px";
  };
  var selfFit = this.fit.bind(this);
  window.addEventListener("resize", selfFit);
  this.fit();

  this.keys = {}; this.kp = {}; this.kr = {};
  window.addEventListener("keydown", function (e) {
    var k = normKey(e.key);
    if (!self.keys[k]) self.kp[k] = 1;
    self.keys[k] = 1;
    if (["space", "left", "right", "up", "down", "tab"].indexOf(k) >= 0) e.preventDefault();
  });
  window.addEventListener("keyup", function (e) {
    var k = normKey(e.key);
    self.keys[k] = 0;
    self.kr[k] = 1;
  });
  this.mouse = { x: 0, y: 0, down: 0, clicked: 0 };
  function pos(e) {
    var r = self.canvas.getBoundingClientRect();
    return { x: (e.clientX - r.left) / r.width * self.canvas.width, y: (e.clientY - r.top) / r.height * self.canvas.height };
  }
  this.canvas.addEventListener("mousemove", function (e) { var p = pos(e); self.mouse.x = p.x; self.mouse.y = p.y; });
  this.canvas.addEventListener("mousedown", function (e) { var p = pos(e); self.mouse.x = p.x; self.mouse.y = p.y; self.mouse.down = 1; self.mouse.clicked = 1; });
  window.addEventListener("mouseup", function () { self.mouse.down = 0; });
  this.canvas.addEventListener("touchstart", function (e) { var t = e.touches[0]; var p = pos(t); self.mouse.x = p.x; self.mouse.y = p.y; self.mouse.down = 1; self.mouse.clicked = 1; e.preventDefault(); }, { passive: false });
  this.canvas.addEventListener("touchmove", function (e) { var t = e.touches[0]; var p = pos(t); self.mouse.x = p.x; self.mouse.y = p.y; e.preventDefault(); }, { passive: false });
  this.canvas.addEventListener("touchend", function () { self.mouse.down = 0; });

  this.warned = {};
  this.playing = [];
  this.start();
}

Runtime.prototype.warnOnce = function (k) {
  if (!this.warned[k]) { this.warned[k] = 1; if (global.console) console.warn("[C3R] non supporté : " + k); }
};

Runtime.prototype.start = function () {
  var lname = this.p.first_layout || (this.p.layouts[0] && this.p.layouts[0].name);
  this.loadLayout(lname);
  this.last = performance.now();
  this.time = 0;
  var self = this;
  requestAnimationFrame(function (t) { self.frame(t); });
};

Runtime.prototype.loadLayout = function (name) {
  this.layoutName = name;
  this.layout = null;
  var self = this;
  (this.p.layouts || []).forEach(function (l) { if (l.name === name) self.layout = l; });
  if (!this.layout && (this.p.layouts || []).length) this.layout = this.p.layouts[0];
  this.instances = [];
  this.created = [];
  this.destroyed = [];
  this.pendingLayout = null;
  this.justStarted = true;
  this.condAcc = {};
  this.camera = { x: 0, y: 0 };
  this.global = {};
  (this.p.global_vars || []).forEach(function (v) { self.global[v.name] = v.initial; });
  (this.layout.layers || []).forEach(function (ly, li) {
    (ly.instances || []).forEach(function (d) { self.spawnInstance(d, li); });
  });
};

Runtime.prototype.spawnInstance = function (d, layerIdx) {
  var t = this.types[d.object];
  if (!t) { this.warnOnce("type:" + d.object); return null; }
  return this.makeInstance(t, d, layerIdx);
};

Runtime.prototype.makeInstance = function (t, d, layerIdx) {
  var self = this;
  var inst = {
    uid: (d && d.uid) || 0,
    type: t.name,
    t: t,
    x: n(d && d.x), y: n(d && d.y),
    width: n(d && d.width, 32), height: n(d && d.height, 32),
    angle: n(d && d.angle), z: n(d && d.z),
    opacity: (d && d.opacity != null) ? n(d.opacity, 100) : 100,
    layer: layerIdx || 0,
    vars: {},
    behaviors: [],
    anim: { name: null, frame: 0, time: 0 },
    timers: {},
    props: Object.assign({}, (t && t.properties) || {}, (d && d.properties) || {}),
    dead: false,
    created: true
  };
  (t.instance_vars || []).forEach(function (v) {
    inst.vars[v.name] = (d && d.vars && d.vars[v.name] !== undefined) ? d.vars[v.name] : v.initial;
  });
  (t.behaviors || []).forEach(function (b) { inst.behaviors.push(new Behavior(b)); });
  var an0 = (t.animations && t.animations[0]) || null;
  inst.anim.name = an0 ? an0.name : null;
  if (inst.width <= 0) inst.width = 32;
  if (inst.height <= 0) inst.height = 32;
  this.instances.push(inst);
  this.created.push(inst);
  return inst;
};

Runtime.prototype.createInstance = function (typeName, x, y, layerIdx) {
  var t = this.types[typeName];
  if (!t) { this.warnOnce("type:" + typeName); return null; }
  var w = 32, h = 32;
  var txid = t.texture || (t.properties && t.properties.texture) || "";
  if (!txid && t.animations && t.animations[0] && t.animations[0].frames && t.animations[0].frames[0]) {
    txid = t.animations[0].frames[0].texture;
  }
  var im = txid ? this.assets[txid] : null;
  if (im && im.width) { w = im.width; h = im.height; }
  return this.makeInstance(t, { x: x, y: y, width: w, height: h }, layerIdx != null ? layerIdx : 0);
};

Runtime.prototype.instancesOf = function (name) {
  var out = [];
  for (var i = 0; i < this.instances.length; i++) {
    var it = this.instances[i];
    if (!it.dead && it.type === name) out.push(it);
  }
  return out;
};
Runtime.prototype.pickList = function (ctx, name) {
  return (ctx && ctx.picked[name]) || this.instancesOf(name);
};

Runtime.prototype.frame = function (now) {
  var dt = Math.min((now - this.last) / 1000, 0.05);
  if (dt < 0) dt = 0;
  this.last = now;
  this.time += dt;
  this.tick(dt);
  this.render();
  this.kp = {}; this.kr = {}; this.mouse.clicked = 0;
  var self = this;
  this.instances.forEach(function (i) { i.created = false; });
  this.created = []; this.destroyed = [];
  if (this.pendingLayout) {
    var nl = this.pendingLayout;
    this.pendingLayout = null;
    this.loadLayout(nl);
  }
  requestAnimationFrame(function (t) { self.frame(t); });
};

Runtime.prototype.tick = function (dt) {
  var self = this;
  this.lastDt = dt;
  this.solids = this.instances.filter(function (i) {
    return !i.dead && i.behaviors.some(function (b) { return b.id === "solid"; });
  });
  this.instances.forEach(function (i) {
    if (i.dead) return;
    self.updateAnims(i, dt);
    self.updateBehaviors(i, dt);
  });
  var sheet = this.sheets[this.layout && this.layout.event_sheet];
  this.picked = {};
  if (sheet) this.runEvents(sheet.events || [], null);
  // caméra : premier ScrollTo trouvé
  var sc = null;
  for (var i = 0; i < this.instances.length && !sc; i++) {
    if (!this.instances[i].dead && this.instances[i].behaviors.some(function (b) { return b.id === "scrollto"; })) sc = this.instances[i];
  }
  if (sc) {
    this.camera.x = sc.x + sc.width / 2 - this.canvas.width / 2;
    this.camera.y = sc.y + sc.height / 2 - this.canvas.height / 2;
    this.camera.x = this.layout.width > this.canvas.width ? Math.max(0, Math.min(this.camera.x, this.layout.width - this.canvas.width)) : (this.layout.width - this.canvas.width) / 2;
    this.camera.y = this.layout.height > this.canvas.height ? Math.max(0, Math.min(this.camera.y, this.layout.height - this.canvas.height)) : (this.layout.height - this.canvas.height) / 2;
  }
  this.instances = this.instances.filter(function (i) { return !i.dead; });
  this.justStarted = false;
};

Runtime.prototype.updateAnims = function (i, dt) {
  var t = i.t;
  if (t.plugin !== "sprite" || !t.animations || !t.animations.length) return;
  var an = null;
  for (var k = 0; k < t.animations.length; k++) if (t.animations[k].name === i.anim.name) an = t.animations[k];
  if (!an) an = t.animations[0];
  if (!an.frames || !an.frames.length) return;
  var fps = n(an.speed, 5);
  if (fps <= 0) return;
  i.anim.time += dt;
  var adv = i.anim.time * fps;
  if (adv >= 1) {
    i.anim.frame += Math.floor(adv);
    i.anim.time -= Math.floor(adv) / fps;
    if (i.anim.frame >= an.frames.length) {
      i.anim.frame = an.looping ? i.anim.frame % an.frames.length : an.frames.length - 1;
    }
  }
};

Runtime.prototype.updateBehaviors = function (i, dt) {
  for (var k = 0; k < i.behaviors.length; k++) {
    var b = i.behaviors[k];
    switch (b.id) {
      case "platform": this.bhPlatform(i, b, dt); break;
      case "eightdir": this.bhEightDir(i, b, dt); break;
      case "bullet": this.bhBullet(i, b, dt); break;
      case "destroyoutside": this.bhDestroyOutside(i, b); break;
      case "fade": this.bhFade(i, b, dt); break;
      case "sine": this.bhSine(i, b, dt); break;
      case "wrap": this.bhWrap(i, b); break;
      case "pin": this.bhPin(i, b); break;
      case "dragndrop": this.bhDrag(i, b); break;
      default: break;
    }
  }
};

Runtime.prototype.bhPlatform = function (i, b, dt) {
  var st = b.state;
  if (!st.init) { st.vx = 0; st.vy = 0; st.floor = false; st.init = true; }
  var maxs = b.num(["max-speed", "maxspeed", "speed"], 330);
  var grav = b.num(["gravity"], 1500);
  var jump = b.num(["jump-strength", "jumpstrength", "jump"], 850);
  var acc = b.num(["acceleration"], 1500);
  var dec = b.num(["deceleration"], 1500);
  var dir = 0;
  if (this.keys["left"]) dir -= 1;
  if (this.keys["right"]) dir += 1;
  if (dir !== 0) {
    st.vx += dir * acc * dt;
    if (st.vx > maxs) st.vx = maxs;
    if (st.vx < -maxs) st.vx = -maxs;
  } else {
    var dv = dec * dt;
    if (st.vx > 0) st.vx = Math.max(0, st.vx - dv);
    else st.vx = Math.min(0, st.vx + dv);
  }
  if ((this.kp["up"] || this.kp["space"]) && st.floor) { st.vy = -jump; st.floor = false; }
  st.vy += grav * dt;
  if (st.vy > 1600) st.vy = 1600;
  i.x += st.vx * dt;
  for (var a = 0; a < this.solids.length; a++) {
    var sol = this.solids[a];
    if (sol !== i && !sol.dead && aabb(i, sol)) {
      if (st.vx > 0) i.x = sol.x - i.width;
      else if (st.vx < 0) i.x = sol.x + sol.width;
      st.vx = 0;
    }
  }
  i.y += st.vy * dt;
  st.floor = false;
  for (var c = 0; c < this.solids.length; c++) {
    var sol2 = this.solids[c];
    if (sol2 !== i && !sol2.dead && aabb(i, sol2)) {
      if (st.vy > 0) { i.y = sol2.y - i.height; st.vy = 0; st.floor = true; }
      else if (st.vy < 0) { i.y = sol2.y + sol2.height; st.vy = 0; }
    }
  }
};

Runtime.prototype.bhEightDir = function (i, b, dt) {
  var st = b.state;
  var sp = b.num(["max-speed", "maxspeed", "speed"], 200);
  var vx = 0, vy = 0;
  if (this.keys["left"]) vx -= sp;
  if (this.keys["right"]) vx += sp;
  if (this.keys["up"]) vy -= sp;
  if (this.keys["down"]) vy += sp;
  if (vx || vy) { st.vx = vx; st.vy = vy; }
  if (st.vx || st.vy) { i.x += st.vx * dt; i.y += st.vy * dt; }
};

Runtime.prototype.bhBullet = function (i, b, dt) {
  var sp = b.state.speed != null ? n(b.state.speed, 400) : b.num(["speed"], 400);
  var r = i.angle * Math.PI / 180;
  i.x += Math.cos(r) * sp * dt;
  i.y += Math.sin(r) * sp * dt;
};

Runtime.prototype.bhDestroyOutside = function (i, b) {
  var m = b.num(["margin", "distance"], 100);
  var L = this.layout;
  if (i.x + i.width < -m || i.x > L.width + m || i.y + i.height < -m || i.y > L.height + m) {
    i.dead = true;
    this.destroyed.push(i);
  }
};

Runtime.prototype.bhFade = function (i, b, dt) {
  var st = b.state;
  st.t = (st.t || 0) + dt;
  var fi = b.num(["fade-in", "fadein"], 0.5);
  var wt = b.num(["wait", "wait-time"], 0);
  var fo = b.num(["fade-out", "fadeout"], 0.5);
  if (st.t < fi) i.opacity = 100 * (st.t / fi);
  else if (st.t < fi + wt) i.opacity = 100;
  else if (st.t < fi + wt + fo) i.opacity = 100 * (1 - (st.t - fi - wt) / fo);
  else { i.dead = true; this.destroyed.push(i); }
};

Runtime.prototype.bhSine = function (i, b, dt) {
  var st = b.state;
  if (!st.base) st.base = { x: i.x, y: i.y };
  st.t = (st.t || 0) + dt;
  var per = b.num(["period"], 1) || 1;
  var rg = b.num(["range"], 50);
  var mv = (b.str(["movement", "movement-type", "movementtype"], "horizontal") || "").toLowerCase();
  var off = Math.sin(2 * Math.PI * st.t / per) * rg;
  if (mv.charAt(0) === "v") i.y = st.base.y + off;
  else i.x = st.base.x + off;
};

Runtime.prototype.bhWrap = function (i, b) {
  var L = this.layout;
  if (i.x + i.width < 0) i.x = L.width;
  else if (i.x > L.width) i.x = -i.width;
  if (i.y + i.height < 0) i.y = L.height;
  else if (i.y > L.height) i.y = -i.height;
};

Runtime.prototype.bhPin = function (i, b) {
  var st = b.state;
  var tgt = st.target;
  if (tgt && !tgt.dead) {
    if (!st.off) st.off = { dx: i.x - tgt.x, dy: i.y - tgt.y };
    i.x = tgt.x + st.off.dx;
    i.y = tgt.y + st.off.dy;
  }
};

Runtime.prototype.bhDrag = function (i, b) {
  var st = b.state;
  if (this.mouse.down && pointIn(this.mouse, i)) st.drag = true;
  if (!this.mouse.down) st.drag = false;
  if (st.drag) { i.x = this.mouse.x - i.width / 2; i.y = this.mouse.y - i.height / 2; }
};

Runtime.prototype.runEvents = function (events, parentCtx) {
  var prevOk = false;
  for (var ei = 0; ei < events.length; ei++) {
    var e = events[ei];
    if (e.disabled) continue;
    var ctx = { picked: {}, parent: parentCtx };
    var ok = true;
    var conds = e.conditions || [];
    if (conds.length === 1 && conds[0].id === "else") {
      ok = !prevOk;
    } else {
      for (var ci = 0; ci < conds.length; ci++) {
        if (!this.testCond(conds[ci], ctx)) { ok = false; break; }
      }
    }
    prevOk = ok;
    if (ok) {
      var acts = e.actions || [];
      for (var ai = 0; ai < acts.length; ai++) this.runAction(acts[ai], ctx);
      if (e.children && e.children.length) this.runEvents(e.children, ctx);
    }
  }
};

Runtime.prototype.testCond = function (c, ctx) {
  var self = this;
  var id = c.id, p = c.params || [], obj = c.object || "system";
  switch (id) {
    case "on_start_of_layout": return this.justStarted;
    case "every_tick": return true;
    case "every_x_seconds": {
      var key = "acc:" + JSON.stringify(c);
      this.condAcc[key] = (this.condAcc[key] || 0) + (this.lastDt || 0);
      if (this.condAcc[key] >= n(p[0], 1)) { this.condAcc[key] = 0; return true; }
      return false;
    }
    case "key_pressed": {
      var ks = keyNames(p[0]);
      return ks.some(function (k) { return self.kp[k]; });
    }
    case "key_down": {
      var kd = keyNames(p[0]);
      return kd.some(function (k) { return self.keys[k]; });
    }
    case "key_released": {
      var kr = keyNames(p[0]);
      return kr.some(function (k) { return self.kr[k]; });
    }
    case "on_object_clicked": {
      if (obj === "system") return this.mouse.clicked;
      var list = this.pickList(ctx, obj).filter(function (i) { return pointIn(self.mouse, i); });
      if (list.length) { ctx.picked[obj] = list; return true; }
      return false;
    }
    case "on_created": {
      var created = this.created.filter(function (i) { return i.type === obj; });
      if (created.length) { ctx.picked[obj] = created; return true; }
      return false;
    }
    case "on_destroyed": {
      var dest = this.destroyed.filter(function (i) { return i.type === obj; });
      return dest.length > 0;
    }
    case "on_collision_with": {
      var target = s(p[0]);
      var A = this.pickList(ctx, obj);
      var B = this.instancesOf(target);
      var hits = [], bhits = [];
      for (var x = 0; x < A.length; x++) {
        for (var y = 0; y < B.length; y++) {
          if (A[x] !== B[y] && aabb(A[x], B[y])) {
            if (hits.indexOf(A[x]) < 0) hits.push(A[x]);
            if (bhits.indexOf(B[y]) < 0) bhits.push(B[y]);
          }
        }
      }
      if (hits.length) { ctx.picked[obj] = hits; ctx.picked[target] = bhits; return true; }
      return false;
    }
    case "compare_instance_var": {
      var vn = s(p[0]);
      var fl = this.pickList(ctx, obj).filter(function (i) { return cmp(i.vars[vn], p[1], p[2]); });
      if (fl.length) { ctx.picked[obj] = fl; return true; }
      return false;
    }
    case "compare_global_var": return cmp(this.global[s(p[0])], p[1], p[2]);
    case "compare_x": {
      var fx = this.pickList(ctx, obj).filter(function (i) { return cmp(i.x, p[0], p[1]); });
      if (fx.length) { ctx.picked[obj] = fx; return true; }
      return false;
    }
    case "compare_y": {
      var fy = this.pickList(ctx, obj).filter(function (i) { return cmp(i.y, p[0], p[1]); });
      if (fy.length) { ctx.picked[obj] = fy; return true; }
      return false;
    }
    case "is_on_screen": {
      var view = { x: this.camera.x, y: this.camera.y, width: this.canvas.width, height: this.canvas.height };
      var on = this.pickList(ctx, obj).filter(function (i) { return aabb(i, view); });
      if (on.length) { ctx.picked[obj] = on; return true; }
      return false;
    }
    case "random_chance": return Math.random() * 100 < n(p[0], 50);
    case "on_timer": {
      var tn = s(p[0]);
      var tl = this.pickList(ctx, obj).filter(function (i) {
        if (i.timers[tn] !== undefined && self.time >= n(i.timers[tn])) { delete i.timers[tn]; return true; }
        return false;
      });
      if (tl.length) { ctx.picked[obj] = tl; return true; }
      return false;
    }
    case "else": return true;
    default:
      this.warnOnce("cond:" + id);
      return false;
  }
};

Runtime.prototype.findAudio = function (x) {
  var want = s(x);
  var list = this.p.assets || [];
  for (var i = 0; i < list.length; i++) {
    var a = list[i];
    if (a.kind === "audio" && (a.id === want || a.name === want)) return a;
  }
  for (var j = 0; j < list.length; j++) {
    if (list[j].kind === "audio") return list[j];
  }
  return null;
};

Runtime.prototype.runAction = function (a, ctx) {
  var self = this;
  var id = a.id, p = a.params || [], obj = a.object || "system";
  var targets = obj === "system" ? [] : this.pickList(ctx, obj);
  function each(f) { targets.forEach(function (i) { if (!i.dead) f(i); }); }
  function f0(i) { return n(p[i]); }
  function s0(i) { return s(p[i]); }
  function pick(tn, inst) { ctx.picked[tn] = (ctx.picked[tn] || []).concat([inst]); }
  switch (id) {
    case "create_object": {
      var tn = s0(0);
      var li = 0;
      if (s0(1)) {
        for (var k = 0; k < this.layout.layers.length; k++) if (this.layout.layers[k].name === s0(1)) li = k;
      }
      var inst = this.createInstance(tn, f0(2), f0(3), li);
      if (inst) pick(tn, inst);
      break;
    }
    case "spawn": {
      var tn2 = s0(0);
      each(function (i) {
        var ni = self.createInstance(tn2, i.x, i.y, i.layer);
        if (ni) pick(tn2, ni);
      });
      break;
    }
    case "destroy":
      each(function (i) { i.dead = true; self.destroyed.push(i); });
      break;
    case "set_x": each(function (i) { i.x = f0(0); }); break;
    case "set_y": each(function (i) { i.y = f0(0); }); break;
    case "set_position": each(function (i) { i.x = f0(0); i.y = f0(1); }); break;
    case "set_angle": each(function (i) { i.angle = f0(0); }); break;
    case "set_width": each(function (i) { i.width = f0(0); }); break;
    case "set_height": each(function (i) { i.height = f0(0); }); break;
    case "set_opacity": each(function (i) { i.opacity = Math.max(0, Math.min(100, f0(0))); }); break;
    case "set_z": each(function (i) { i.z = f0(0); }); break;
    case "set_instance_var": {
      var vn = s0(0);
      each(function (i) { i.vars[vn] = applyOp(i.vars[vn], p[1], p[2]); });
      break;
    }
    case "set_global_var": {
      var gn = s0(0);
      this.global[gn] = applyOp(this.global[gn], p[1], p[2]);
      break;
    }
    case "set_text":
      each(function (i) { i.props.text = s0(0); });
      break;
    case "set_font_size":
      each(function (i) { i.props.size = f0(0); });
      break;
    case "set_animation":
      each(function (i) {
        if (i.anim.name !== s0(0)) { i.anim.name = s0(0); i.anim.frame = 0; i.anim.time = 0; }
      });
      break;
    case "set_animation_frame":
      each(function (i) { i.anim.frame = f0(0); i.anim.time = 0; });
      break;
    case "move_forward":
      each(function (i) {
        var r = i.angle * Math.PI / 180;
        i.x += Math.cos(r) * f0(0);
        i.y += Math.sin(r) * f0(0);
      });
      break;
    case "go_to_layout": this.pendingLayout = s0(0); break;
    case "restart_layout": this.pendingLayout = this.layoutName; break;
    case "scroll_to": each(function (i) { i.behaviors.push(new Behavior({ id: "scrollto", name: "ScrollTo" })); }); break;
    case "play_sound": {
      var asset = this.findAudio(p[0]);
      if (asset && asset.data) {
        try {
          var au = new Audio(asset.data);
          au.loop = n(p[1], 0) !== 0;
          au.volume = Math.max(0, Math.min(1, n(p[2], 1)));
          au.play().catch(function () {});
          this.playing.push(au);
        } catch (e) { /* audio indisponible */ }
      } else this.warnOnce("son:" + s0(0));
      break;
    }
    case "set_eightdir_velocity":
      each(function (i) {
        for (var q = 0; q < i.behaviors.length; q++) {
          if (i.behaviors[q].id === "eightdir") { i.behaviors[q].state.vx = f0(0); i.behaviors[q].state.vy = f0(1); }
        }
      });
      break;
    case "set_bullet_speed":
      each(function (i) {
        for (var q = 0; q < i.behaviors.length; q++) {
          if (i.behaviors[q].id === "bullet") i.behaviors[q].state.speed = f0(0);
        }
      });
      break;
    case "start_timer":
      each(function (i) { i.timers[s0(0)] = this.time + f0(1); }.bind(this));
      break;
    case "pin_to": {
      var tgt = this.instancesOf(s0(0))[0];
      each(function (i) {
        for (var q = 0; q < i.behaviors.length; q++) {
          if (i.behaviors[q].id === "pin") i.behaviors[q].state.target = tgt;
        }
      });
      break;
    }
    case "wait": this.warnOnce("action:wait"); break;
    default: this.warnOnce("action:" + id);
  }
};

Runtime.prototype.currentFrameTexture = function (i) {
  var t = i.t;
  if (!t.animations || !t.animations.length) return "";
  var an = null;
  for (var k = 0; k < t.animations.length; k++) if (t.animations[k].name === i.anim.name) an = t.animations[k];
  if (!an) an = t.animations[0];
  if (!an.frames || !an.frames.length) return "";
  var fr = an.frames[Math.min(i.anim.frame, an.frames.length - 1)];
  return fr ? fr.texture : "";
};

Runtime.prototype.render = function () {
  var ctx = this.ctx;
  var self = this;
  ctx.setTransform(1, 0, 0, 1, 0, 0);
  ctx.fillStyle = (this.layout && this.layout.background) || "#ffffff";
  ctx.fillRect(0, 0, this.canvas.width, this.canvas.height);
  var layers = (this.layout && this.layout.layers) || [];
  for (var li = 0; li < layers.length; li++) {
    var ly = layers[li];
    if (ly.visible === false) continue;
    ctx.globalAlpha = Math.max(0, Math.min(1, (ly.opacity == null ? 100 : ly.opacity) / 100));
    var px = (ly.parallax_x == null ? 100 : ly.parallax_x) / 100;
    var py = (ly.parallax_y == null ? 100 : ly.parallax_y) / 100;
    var ox = -this.camera.x * px, oy = -this.camera.y * py;
    var insts = this.instances.filter(function (i) { return i.layer === li && !i.dead; });
    insts.sort(function (a, b) { return a.z - b.z; });
    for (var q = 0; q < insts.length; q++) this.drawInst(ctx, insts[q], ox, oy);
  }
  ctx.globalAlpha = 1;
};

Runtime.prototype.drawInst = function (ctx, i, ox, oy) {
  var self = this;
  var t = i.t;
  if (t.plugin === "tiledbg") {
    var im = this.assets[t.texture || (t.properties && t.properties.texture)];
    if (im && im.width) {
      ctx.save();
      ctx.translate(i.x + ox, i.y + oy);
      var pat = ctx.createPattern(im, "repeat");
      ctx.fillStyle = pat;
      ctx.fillRect(0, 0, i.width, i.height);
      ctx.restore();
    } else {
      ctx.fillStyle = "#9a9a9a";
      ctx.fillRect(i.x + ox, i.y + oy, i.width, i.height);
    }
    return;
  }
  if (t.plugin === "text") {
    var txt = String(i.props.text == null ? "" : i.props.text).replace(/\{(\w+)\}/g, function (_, vn) {
      var v = self.global[vn];
      return v === undefined ? "{" + vn + "}" : String(v);
    });
    var size = n(i.props.size, 18);
    ctx.save();
    ctx.translate(i.x + ox, i.y + oy);
    ctx.fillStyle = i.props.color || "#000000";
    ctx.font = (i.props.bold ? "bold " : "") + size + "px Arial, sans-serif";
    ctx.textBaseline = "top";
    var lines = txt.split("\n");
    for (var li = 0; li < lines.length; li++) ctx.fillText(lines[li], 0, li * size * 1.25);
    ctx.restore();
    return;
  }
  var fid = this.currentFrameTexture(i);
  var im2 = fid ? this.assets[fid] : null;
  ctx.save();
  ctx.translate(i.x + i.width / 2 + ox, i.y + i.height / 2 + oy);
  ctx.rotate(i.angle * Math.PI / 180);
  ctx.globalAlpha *= Math.max(0, Math.min(1, i.opacity / 100));
  if (im2 && im2.width) {
    ctx.drawImage(im2, -i.width / 2, -i.height / 2, i.width, i.height);
  } else {
    ctx.fillStyle = "#b8bcc4";
    ctx.fillRect(-i.width / 2, -i.height / 2, i.width, i.height);
    ctx.strokeStyle = "#7a7f88";
    ctx.strokeRect(-i.width / 2, -i.height / 2, i.width, i.height);
  }
  ctx.restore();
};

global.C3R_Runtime = Runtime;
})(typeof window !== "undefined" ? window : globalThis);
