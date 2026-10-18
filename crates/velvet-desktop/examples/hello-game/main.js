// Hello Velvet: a small game page that uses every part of window.velvet.
(function () {
  "use strict";

  var SLOT = "hello";
  var output = document.getElementById("output");
  var saveCount = 0;

  function say(text) {
    output.textContent += text + "\n";
  }

  function errorText(error) {
    return error && error.code ? error.code + ": " + error.message : String(error);
  }

  var velvet = window.velvet;
  if (!velvet) {
    say("window.velvet is missing. Start this game with: cargo run -p velvet-desktop -- <folder>");
    return;
  }

  document.getElementById("title").textContent = velvet.game.title;
  say("Bridge ready: " + velvet.platform + ", " + velvet.game.id + " " + velvet.game.version);

  var actions = {
    info: function () {
      say(JSON.stringify({ platform: velvet.platform, game: velvet.game }, null, 2));
      return Promise.resolve();
    },
    save: function () {
      saveCount += 1;
      var text = JSON.stringify({ savedAt: new Date().toISOString(), count: saveCount });
      return velvet.saves.write(SLOT, text).then(function () {
        say("Saved slot " + SLOT + ": " + text);
      });
    },
    load: function () {
      return velvet.saves.read(SLOT).then(function (text) {
        say(text === null ? "Slot " + SLOT + " is empty." : "Loaded slot " + SLOT + ": " + text);
      });
    },
    list: function () {
      return velvet.saves.list().then(function (slots) {
        say("Save slots: " + (slots.length ? slots.join(", ") : "(none)"));
      });
    },
    remove: function () {
      return velvet.saves.remove(SLOT).then(function () {
        say("Removed slot " + SLOT + ".");
      });
    },
    fullscreen: function () {
      return velvet.window
        .isFullscreen()
        .then(function (on) {
          return velvet.window.setFullscreen(!on);
        })
        .then(function () {
          return velvet.window.isFullscreen();
        })
        .then(function (on) {
          say("Fullscreen is now " + on + ".");
        });
    },
    log: function () {
      velvet.log("info", "Hello from main.js at " + new Date().toLocaleTimeString());
      velvet.log("warn", "This warning goes to the terminal, not to the page.");
      say("Sent two log lines (look at the terminal).");
      return Promise.resolve();
    }
  };

  function run(name) {
    Promise.resolve()
      .then(function () {
        return actions[name]();
      })
      .catch(function (error) {
        say("Error: " + errorText(error));
      });
  }

  document.querySelectorAll("button[data-action]").forEach(function (button) {
    button.addEventListener("click", function () {
      run(button.dataset.action);
    });
  });
})();
