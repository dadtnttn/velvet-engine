//! VS2 host runtime: story presentation, layers, i18n hooks without Python.

#![allow(missing_docs)]
#![allow(dead_code)]

use std::collections::HashMap;
use velvet_script_bytecode::opcodes_vs2::OpVs2;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DialogueLine { pub speaker: String, pub text: String, pub msg_id: Option<String> }

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MenuChoice { pub label: String, pub index: u32 }

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StageChar { pub name: String, pub at: Option<String>, pub visible: bool }

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LayerEntry { pub id: String, pub visible: bool, pub z: i32 }

#[derive(Debug, Clone, Default)]
pub struct Vs2Host {
    pub pool: Vec<String>,
    pub dialogue: Vec<DialogueLine>,
    pub pending_menu: Vec<MenuChoice>,
    pub characters: HashMap<String, StageChar>,
    pub background: Option<String>,
    pub music: Option<String>,
    pub layers: Vec<LayerEntry>,
    pub state: HashMap<String, String>,
    pub translations: HashMap<String, String>,
    pub locale: String,
    pub log: Vec<String>,
    pub await_clicks: u32,
    pub yielded: bool,
}

impl Vs2Host {
    pub fn new() -> Self { Self { locale: "en".into(), ..Default::default() } }
    pub fn with_pool(pool: Vec<String>) -> Self { let mut h = Self::new(); h.pool = pool; h }
    pub fn pool_str(&self, id: u32) -> String { self.pool.get(id as usize).cloned().unwrap_or_default() }
    pub fn set_translation(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.translations.insert(key.into(), value.into());
    }
    pub fn t(&self, key: &str) -> String {
        self.translations.get(key).cloned().unwrap_or_else(|| format!("[{key}]"))
    }
    pub fn push_layer(&mut self, id: impl Into<String>) {
        let id = id.into();
        self.layers.push(LayerEntry { id: id.clone(), visible: true, z: self.layers.len() as i32 });
        self.log.push(format!("push_layer {id}"));
    }
    pub fn pop_layer(&mut self) -> Option<LayerEntry> {
        let e = self.layers.pop();
        if let Some(ref e) = e { self.log.push(format!("pop_layer {}", e.id)); }
        e
    }
    pub fn show_layer(&mut self, id: &str) {
        if let Some(l) = self.layers.iter_mut().find(|l| l.id == id) { l.visible = true; }
        self.log.push(format!("show_layer {id}"));
    }
    pub fn hide_layer(&mut self, id: &str) {
        if let Some(l) = self.layers.iter_mut().find(|l| l.id == id) { l.visible = false; }
        self.log.push(format!("hide_layer {id}"));
    }
    pub fn set_layer_z(&mut self, id: &str, z: i32) {
        if let Some(l) = self.layers.iter_mut().find(|l| l.id == id) { l.z = z; }
    }
    pub fn say(&mut self, speaker: &str, text: &str) {
        self.dialogue.push(DialogueLine { speaker: speaker.into(), text: text.into(), msg_id: None });
        self.log.push(format!("say {speaker}: {text}"));
    }
    pub fn say_msg(&mut self, speaker: &str, msg_id: &str) {
        let text = self.t(msg_id);
        self.dialogue.push(DialogueLine { speaker: speaker.into(), text, msg_id: Some(msg_id.into()) });
        self.log.push(format!("say_msg {speaker}: {msg_id}"));
    }
    pub fn show_char(&mut self, name: &str, at: Option<&str>) {
        self.characters.insert(name.into(), StageChar { name: name.into(), at: at.map(|s| s.into()), visible: true });
    }
    pub fn hide_char(&mut self, name: &str) {
        if let Some(c) = self.characters.get_mut(name) { c.visible = false; }
    }
    pub fn set_bg(&mut self, name: &str) { self.background = Some(name.into()); }
    pub fn set_music(&mut self, name: &str) { self.music = Some(name.into()); }
    pub fn store_state(&mut self, key: &str, val: &str) { self.state.insert(key.into(), val.into()); }
    pub fn load_state(&self, key: &str) -> Option<&str> { self.state.get(key).map(|s| s.as_str()) }
    pub fn exec_op(&mut self, op: OpVs2, a: u32, b: u32, stack_top: Option<&str>) {
        match op {
            OpVs2::Say => {
                let speaker = self.pool_str(a);
                let text = stack_top.unwrap_or("").to_string();
                self.say(&speaker, &text);
            }
            OpVs2::LoadMsg | OpVs2::Translate => { let _ = self.t(&self.pool_str(a)); }
            OpVs2::Menu => { self.pending_menu.clear(); self.log.push(format!("menu choices={a}")); }
            OpVs2::Choice => {
                let label = self.pool_str(a);
                self.pending_menu.push(MenuChoice { label, index: b });
            }
            OpVs2::ShowChar => {
                let name = self.pool_str(a);
                let at = if b == 0 { None } else { Some(self.pool_str(b)) };
                self.show_char(&name, at.as_deref());
            }
            OpVs2::HideChar => self.hide_char(&self.pool_str(a)),
            OpVs2::Background => self.set_bg(&self.pool_str(a)),
            OpVs2::Music => self.set_music(&self.pool_str(a)),
            OpVs2::PushLayer => self.push_layer(self.pool_str(a)),
            OpVs2::PopLayer => { let _ = self.pop_layer(); }
            OpVs2::ShowLayer => self.show_layer(&self.pool_str(a)),
            OpVs2::HideLayer => self.hide_layer(&self.pool_str(a)),
            OpVs2::SetLayerZ => self.set_layer_z(&self.pool_str(a), b as i32),
            OpVs2::StoreState => self.store_state(&self.pool_str(a), stack_top.unwrap_or("")),
            OpVs2::Await => self.await_clicks = self.await_clicks.saturating_add(1),
            OpVs2::Yield => self.yielded = true,
            _ => {}
        }
    }
    pub fn visible_layers(&self) -> Vec<&LayerEntry> { self.layers.iter().filter(|l| l.visible).collect() }
    pub fn visible_chars(&self) -> Vec<&StageChar> { self.characters.values().filter(|c| c.visible).collect() }
    pub fn last_line(&self) -> Option<&DialogueLine> { self.dialogue.last() }
    pub fn clear_dialogue(&mut self) { self.dialogue.clear(); }
    pub fn reset_stage(&mut self) {
        self.dialogue.clear(); self.pending_menu.clear(); self.characters.clear();
        self.background = None; self.music = None; self.layers.clear();
        self.yielded = false; self.await_clicks = 0; self.log.clear();
    }
}

#[derive(Debug, Default)]
pub struct Vs2MiniVm {
    pub host: Vs2Host,
    pub stack: Vec<String>,
    pub locals: Vec<String>,
    pub pc: usize,
    pub code: Vec<(OpVs2, u32, u32)>,
    pub halted: bool,
}

impl Vs2MiniVm {
    pub fn new(host: Vs2Host) -> Self { Self { host, ..Default::default() } }
    pub fn load(&mut self, code: Vec<(OpVs2, u32, u32)>) {
        self.code = code; self.pc = 0; self.halted = false; self.stack.clear();
    }
    pub fn push(&mut self, v: impl Into<String>) { self.stack.push(v.into()); }
    pub fn pop(&mut self) -> String { self.stack.pop().unwrap_or_default() }
    pub fn step(&mut self) -> bool {
        if self.halted || self.pc >= self.code.len() { self.halted = true; return false; }
        let (op, a, b) = self.code[self.pc];
        self.pc += 1;
        match op {
            OpVs2::Nop => {}
            OpVs2::LoadConst => {
                let s = self.host.pool_str(a);
                if s.is_empty() { self.push(a.to_string()); } else { self.push(s); }
            }
            OpVs2::LoadLocal => {
                let v = self.locals.get(a as usize).cloned().unwrap_or_default();
                self.push(v);
            }
            OpVs2::StoreLocal => {
                let v = self.pop();
                let idx = a as usize;
                if self.locals.len() <= idx { self.locals.resize(idx + 1, String::new()); }
                self.locals[idx] = v;
            }
            OpVs2::Add => {
                let r = self.pop(); let l = self.pop();
                if let (Ok(li), Ok(ri)) = (l.parse::<i64>(), r.parse::<i64>()) {
                    self.push((li + ri).to_string());
                } else { self.push(format!("{l}{r}")); }
            }
            OpVs2::Sub => {
                let r = self.pop().parse::<i64>().unwrap_or(0);
                let l = self.pop().parse::<i64>().unwrap_or(0);
                self.push((l - r).to_string());
            }
            OpVs2::Mul => {
                let r = self.pop().parse::<i64>().unwrap_or(0);
                let l = self.pop().parse::<i64>().unwrap_or(0);
                self.push((l * r).to_string());
            }
            OpVs2::Div => {
                let r = self.pop().parse::<i64>().unwrap_or(1);
                let l = self.pop().parse::<i64>().unwrap_or(0);
                self.push(if r == 0 { 0 } else { l / r }.to_string());
            }
            OpVs2::Eq => {
                let r = self.pop(); let l = self.pop();
                self.push(if l == r { "1" } else { "0" });
            }
            OpVs2::Ne => {
                let r = self.pop(); let l = self.pop();
                self.push(if l != r { "1" } else { "0" });
            }
            OpVs2::Lt | OpVs2::Le | OpVs2::Gt | OpVs2::Ge => {
                let r = self.pop().parse::<f64>().unwrap_or(0.0);
                let l = self.pop().parse::<f64>().unwrap_or(0.0);
                let ok = match op {
                    OpVs2::Lt => l < r,
                    OpVs2::Le => l <= r,
                    OpVs2::Gt => l > r,
                    OpVs2::Ge => l >= r,
                    _ => false,
                };
                self.push(if ok { "1" } else { "0" });
            }
            OpVs2::And => {
                let r = self.pop();
                let l = self.pop();
                let lt = !(l == "0" || l.is_empty());
                let rt = !(r == "0" || r.is_empty());
                self.push(if lt && rt { "1" } else { "0" });
            }
            OpVs2::Or => {
                let r = self.pop();
                let l = self.pop();
                let lt = !(l == "0" || l.is_empty());
                let rt = !(r == "0" || r.is_empty());
                self.push(if lt || rt { "1" } else { "0" });
            }
            OpVs2::Not => {
                let v = self.pop();
                self.push(if v == "0" || v.is_empty() { "1" } else { "0" });
            }
            OpVs2::Pop => { let _ = self.pop(); }
            OpVs2::Dup => {
                let v = self.stack.last().cloned().unwrap_or_default();
                self.push(v);
            }
            OpVs2::Jump => { self.pc = a as usize; }
            OpVs2::JumpIf => {
                let v = self.pop();
                if v == "0" || v.is_empty() { self.pc = a as usize; }
            }
            OpVs2::Ret => { self.halted = true; }
            OpVs2::Print => {
                let v = self.pop();
                self.host.log.push(format!("print {v}"));
            }
            OpVs2::LoadMsg | OpVs2::Translate => {
                let key = self.host.pool_str(a);
                self.push(self.host.t(&key));
            }
            OpVs2::LoadState => {
                let key = self.host.pool_str(a);
                let v = self.host.load_state(&key).unwrap_or("0").to_string();
                self.push(v);
            }
            OpVs2::StoreState => {
                let key = self.host.pool_str(a);
                let val = self.pop();
                self.host.store_state(&key, &val);
            }
            OpVs2::Call | OpVs2::ActionFire => {
                // Registered story commands (e.g. combat.start) land here after lower.
                let name = self.host.pool_str(a);
                let argc = b;
                let mut args = Vec::new();
                for _ in 0..argc {
                    args.push(self.pop());
                }
                args.reverse();
                let arg_s = args.join(",");
                self.host
                    .log
                    .push(format!("command {name} args=[{arg_s}]"));
                self.host
                    .store_state("__last_command", &name);
                self.host
                    .store_state(&format!("cmd.{name}"), "1");
                // leave a unit-ish result for stack balance
                self.push("ok");
            }
            other => {
                let top = self.stack.last().cloned();
                self.host.exec_op(other, a, b, top.as_deref());
            }
        }
        !self.halted
    }
    pub fn run(&mut self, max_steps: usize) -> usize {
        let mut n = 0;
        while n < max_steps && self.step() { n += 1; }
        n
    }
}














































































































































/// Build a tiny dialogue + layer scenario (one helper).
pub fn scenario(speaker: &str, msg_key: &str, layer: &str, line: &str) -> Vs2MiniVm {
    let mut host = Vs2Host::new();
    host.pool = vec![speaker.into(), msg_key.into(), layer.into()];
    host.set_translation(msg_key, line);
    let mut vm = Vs2MiniVm::new(host);
    vm.load(vec![
        (OpVs2::LoadMsg, 1, 0),
        (OpVs2::Say, 0, 0),
        (OpVs2::PushLayer, 2, 0),
        (OpVs2::Ret, 0, 0),
    ]);
    vm
}

pub fn run_scenario() -> Vs2Host {
    let mut vm = scenario("hero", "k0", "hud", "line-0");
    let _ = vm.run(32);
    vm.host
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn host_say_and_layer() {
        let mut h = Vs2Host::new();
        h.set_translation("dlg.hi", "Hola");
        h.say_msg("eira", "dlg.hi");
        h.push_layer("settings");
        assert_eq!(h.last_line().unwrap().text, "Hola");
        assert_eq!(h.layers.len(), 1);
    }
    #[test]
    fn mini_vm_add() {
        let mut vm = Vs2MiniVm::new(Vs2Host::new());
        vm.load(vec![
            (OpVs2::LoadConst, 2, 0),
            (OpVs2::LoadConst, 3, 0),
            (OpVs2::Add, 0, 0),
            (OpVs2::Ret, 0, 0),
        ]);
        vm.run(16);
        assert_eq!(vm.stack.last().map(|s| s.as_str()), Some("5"));
    }
    #[test]
    fn scenario_runs() {
        let h = run_scenario();
        assert!(!h.dialogue.is_empty());
        assert!(!h.layers.is_empty());
    }
    #[test]
    fn exec_show_hide_char() {
        let mut h = Vs2Host::new();
        h.pool = vec!["hero".into(), "left".into()];
        h.exec_op(OpVs2::ShowChar, 0, 1, None);
        assert!(h.characters["hero"].visible);
        h.exec_op(OpVs2::HideChar, 0, 0, None);
        assert!(!h.characters["hero"].visible);
    }
}

