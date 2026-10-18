// Velvet Desktop bridge: defines window.velvet for the game's own pages.
// The host injects this script before any page script runs (see src/bridge.rs).
// The host replaces the config placeholder in the code below with JSON:
// { "game": { id, title, version }, "origin": "..." }.
(function () {
  "use strict";

  var config = __CONFIG__;

  // Only the game's own pages get the bridge, never an outside page inside a frame.
  if (String(window.location.href).indexOf(config.origin + "/") !== 0) {
    return;
  }
  if (window.velvet) {
    return;
  }

  var pending = new Map();
  var nextId = 1;

  function post(id, cmd, args) {
    window.ipc.postMessage(JSON.stringify({ id: id, cmd: cmd, args: args }));
  }

  function call(cmd, args) {
    return new Promise(function (resolve, reject) {
      var id = nextId++;
      pending.set(id, { resolve: resolve, reject: reject });
      post(id, cmd, args || {});
    });
  }

  // The host calls __velvetReply(id, ok, valueJson) to settle one call.
  Object.defineProperty(window, "__velvetReply", {
    value: function (id, ok, valueJson) {
      var entry = pending.get(id);
      if (!entry) {
        return;
      }
      pending.delete(id);
      var value = JSON.parse(valueJson);
      if (ok) {
        entry.resolve(value);
      } else {
        var error = new Error(value.message);
        error.code = value.code;
        entry.reject(error);
      }
    }
  });

  var bridge = {
    platform: "desktop",
    game: Object.freeze(config.game),
    saves: Object.freeze({
      list: function () { return call("saves.list"); },
      read: function (slot) { return call("saves.read", { slot: slot }); },
      write: function (slot, text) { return call("saves.write", { slot: slot, text: text }); },
      remove: function (slot) { return call("saves.remove", { slot: slot }); }
    }),
    settings: Object.freeze({
      get: function (key) { return call("settings.get", { key: key }); },
      set: function (key, value) { return call("settings.set", { key: key, value: value }); }
    }),
    window: Object.freeze({
      setFullscreen: function (on) { return call("window.setFullscreen", { on: on }); },
      isFullscreen: function () { return call("window.isFullscreen"); },
      close: function () { return call("window.close"); }
    }),
    log: function (level, message) {
      // No reply id: the host writes the line and does not answer.
      post(null, "log", { level: String(level), message: String(message) });
    }
  };

  Object.defineProperty(window, "velvet", { value: Object.freeze(bridge), enumerable: true });
})();
