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

pub fn scenario_0(speaker: &str, msg_key: &str, layer: &str) -> Vs2MiniVm {
    let mut host = Vs2Host::new();
    host.pool = vec![speaker.into(), msg_key.into(), layer.into()];
    host.set_translation(msg_key, format!("line-0"));
    let mut vm = Vs2MiniVm::new(host);
    vm.load(vec![
        (OpVs2::LoadMsg, 1, 0),
        (OpVs2::Say, 0, 0),
        (OpVs2::PushLayer, 2, 0),
        (OpVs2::Ret, 0, 0),
    ]);
    vm
}

pub fn run_scenario_0() -> Vs2Host {
    let mut vm = scenario_0("hero", "k0", "hud");
    let _ = vm.run(32);
    vm.host
}

pub fn scenario_1(speaker: &str, msg_key: &str, layer: &str) -> Vs2MiniVm {
    let mut host = Vs2Host::new();
    host.pool = vec![speaker.into(), msg_key.into(), layer.into()];
    host.set_translation(msg_key, format!("line-1"));
    let mut vm = Vs2MiniVm::new(host);
    vm.load(vec![
        (OpVs2::LoadMsg, 1, 0),
        (OpVs2::Say, 0, 0),
        (OpVs2::PushLayer, 2, 0),
        (OpVs2::Ret, 0, 0),
    ]);
    vm
}

pub fn run_scenario_1() -> Vs2Host {
    let mut vm = scenario_1("hero", "k1", "hud");
    let _ = vm.run(32);
    vm.host
}

pub fn scenario_2(speaker: &str, msg_key: &str, layer: &str) -> Vs2MiniVm {
    let mut host = Vs2Host::new();
    host.pool = vec![speaker.into(), msg_key.into(), layer.into()];
    host.set_translation(msg_key, format!("line-2"));
    let mut vm = Vs2MiniVm::new(host);
    vm.load(vec![
        (OpVs2::LoadMsg, 1, 0),
        (OpVs2::Say, 0, 0),
        (OpVs2::PushLayer, 2, 0),
        (OpVs2::Ret, 0, 0),
    ]);
    vm
}

pub fn run_scenario_2() -> Vs2Host {
    let mut vm = scenario_2("hero", "k2", "hud");
    let _ = vm.run(32);
    vm.host
}

pub fn scenario_3(speaker: &str, msg_key: &str, layer: &str) -> Vs2MiniVm {
    let mut host = Vs2Host::new();
    host.pool = vec![speaker.into(), msg_key.into(), layer.into()];
    host.set_translation(msg_key, format!("line-3"));
    let mut vm = Vs2MiniVm::new(host);
    vm.load(vec![
        (OpVs2::LoadMsg, 1, 0),
        (OpVs2::Say, 0, 0),
        (OpVs2::PushLayer, 2, 0),
        (OpVs2::Ret, 0, 0),
    ]);
    vm
}

pub fn run_scenario_3() -> Vs2Host {
    let mut vm = scenario_3("hero", "k3", "hud");
    let _ = vm.run(32);
    vm.host
}

pub fn scenario_4(speaker: &str, msg_key: &str, layer: &str) -> Vs2MiniVm {
    let mut host = Vs2Host::new();
    host.pool = vec![speaker.into(), msg_key.into(), layer.into()];
    host.set_translation(msg_key, format!("line-4"));
    let mut vm = Vs2MiniVm::new(host);
    vm.load(vec![
        (OpVs2::LoadMsg, 1, 0),
        (OpVs2::Say, 0, 0),
        (OpVs2::PushLayer, 2, 0),
        (OpVs2::Ret, 0, 0),
    ]);
    vm
}

pub fn run_scenario_4() -> Vs2Host {
    let mut vm = scenario_4("hero", "k4", "hud");
    let _ = vm.run(32);
    vm.host
}

pub fn scenario_5(speaker: &str, msg_key: &str, layer: &str) -> Vs2MiniVm {
    let mut host = Vs2Host::new();
    host.pool = vec![speaker.into(), msg_key.into(), layer.into()];
    host.set_translation(msg_key, format!("line-5"));
    let mut vm = Vs2MiniVm::new(host);
    vm.load(vec![
        (OpVs2::LoadMsg, 1, 0),
        (OpVs2::Say, 0, 0),
        (OpVs2::PushLayer, 2, 0),
        (OpVs2::Ret, 0, 0),
    ]);
    vm
}

pub fn run_scenario_5() -> Vs2Host {
    let mut vm = scenario_5("hero", "k5", "hud");
    let _ = vm.run(32);
    vm.host
}

pub fn scenario_6(speaker: &str, msg_key: &str, layer: &str) -> Vs2MiniVm {
    let mut host = Vs2Host::new();
    host.pool = vec![speaker.into(), msg_key.into(), layer.into()];
    host.set_translation(msg_key, format!("line-6"));
    let mut vm = Vs2MiniVm::new(host);
    vm.load(vec![
        (OpVs2::LoadMsg, 1, 0),
        (OpVs2::Say, 0, 0),
        (OpVs2::PushLayer, 2, 0),
        (OpVs2::Ret, 0, 0),
    ]);
    vm
}

pub fn run_scenario_6() -> Vs2Host {
    let mut vm = scenario_6("hero", "k6", "hud");
    let _ = vm.run(32);
    vm.host
}

pub fn scenario_7(speaker: &str, msg_key: &str, layer: &str) -> Vs2MiniVm {
    let mut host = Vs2Host::new();
    host.pool = vec![speaker.into(), msg_key.into(), layer.into()];
    host.set_translation(msg_key, format!("line-7"));
    let mut vm = Vs2MiniVm::new(host);
    vm.load(vec![
        (OpVs2::LoadMsg, 1, 0),
        (OpVs2::Say, 0, 0),
        (OpVs2::PushLayer, 2, 0),
        (OpVs2::Ret, 0, 0),
    ]);
    vm
}

pub fn run_scenario_7() -> Vs2Host {
    let mut vm = scenario_7("hero", "k7", "hud");
    let _ = vm.run(32);
    vm.host
}

pub fn scenario_8(speaker: &str, msg_key: &str, layer: &str) -> Vs2MiniVm {
    let mut host = Vs2Host::new();
    host.pool = vec![speaker.into(), msg_key.into(), layer.into()];
    host.set_translation(msg_key, format!("line-8"));
    let mut vm = Vs2MiniVm::new(host);
    vm.load(vec![
        (OpVs2::LoadMsg, 1, 0),
        (OpVs2::Say, 0, 0),
        (OpVs2::PushLayer, 2, 0),
        (OpVs2::Ret, 0, 0),
    ]);
    vm
}

pub fn run_scenario_8() -> Vs2Host {
    let mut vm = scenario_8("hero", "k8", "hud");
    let _ = vm.run(32);
    vm.host
}

pub fn scenario_9(speaker: &str, msg_key: &str, layer: &str) -> Vs2MiniVm {
    let mut host = Vs2Host::new();
    host.pool = vec![speaker.into(), msg_key.into(), layer.into()];
    host.set_translation(msg_key, format!("line-9"));
    let mut vm = Vs2MiniVm::new(host);
    vm.load(vec![
        (OpVs2::LoadMsg, 1, 0),
        (OpVs2::Say, 0, 0),
        (OpVs2::PushLayer, 2, 0),
        (OpVs2::Ret, 0, 0),
    ]);
    vm
}

pub fn run_scenario_9() -> Vs2Host {
    let mut vm = scenario_9("hero", "k9", "hud");
    let _ = vm.run(32);
    vm.host
}

pub fn scenario_10(speaker: &str, msg_key: &str, layer: &str) -> Vs2MiniVm {
    let mut host = Vs2Host::new();
    host.pool = vec![speaker.into(), msg_key.into(), layer.into()];
    host.set_translation(msg_key, format!("line-10"));
    let mut vm = Vs2MiniVm::new(host);
    vm.load(vec![
        (OpVs2::LoadMsg, 1, 0),
        (OpVs2::Say, 0, 0),
        (OpVs2::PushLayer, 2, 0),
        (OpVs2::Ret, 0, 0),
    ]);
    vm
}

pub fn run_scenario_10() -> Vs2Host {
    let mut vm = scenario_10("hero", "k10", "hud");
    let _ = vm.run(32);
    vm.host
}

pub fn scenario_11(speaker: &str, msg_key: &str, layer: &str) -> Vs2MiniVm {
    let mut host = Vs2Host::new();
    host.pool = vec![speaker.into(), msg_key.into(), layer.into()];
    host.set_translation(msg_key, format!("line-11"));
    let mut vm = Vs2MiniVm::new(host);
    vm.load(vec![
        (OpVs2::LoadMsg, 1, 0),
        (OpVs2::Say, 0, 0),
        (OpVs2::PushLayer, 2, 0),
        (OpVs2::Ret, 0, 0),
    ]);
    vm
}

pub fn run_scenario_11() -> Vs2Host {
    let mut vm = scenario_11("hero", "k11", "hud");
    let _ = vm.run(32);
    vm.host
}

pub fn scenario_12(speaker: &str, msg_key: &str, layer: &str) -> Vs2MiniVm {
    let mut host = Vs2Host::new();
    host.pool = vec![speaker.into(), msg_key.into(), layer.into()];
    host.set_translation(msg_key, format!("line-12"));
    let mut vm = Vs2MiniVm::new(host);
    vm.load(vec![
        (OpVs2::LoadMsg, 1, 0),
        (OpVs2::Say, 0, 0),
        (OpVs2::PushLayer, 2, 0),
        (OpVs2::Ret, 0, 0),
    ]);
    vm
}

pub fn run_scenario_12() -> Vs2Host {
    let mut vm = scenario_12("hero", "k12", "hud");
    let _ = vm.run(32);
    vm.host
}

pub fn scenario_13(speaker: &str, msg_key: &str, layer: &str) -> Vs2MiniVm {
    let mut host = Vs2Host::new();
    host.pool = vec![speaker.into(), msg_key.into(), layer.into()];
    host.set_translation(msg_key, format!("line-13"));
    let mut vm = Vs2MiniVm::new(host);
    vm.load(vec![
        (OpVs2::LoadMsg, 1, 0),
        (OpVs2::Say, 0, 0),
        (OpVs2::PushLayer, 2, 0),
        (OpVs2::Ret, 0, 0),
    ]);
    vm
}

pub fn run_scenario_13() -> Vs2Host {
    let mut vm = scenario_13("hero", "k13", "hud");
    let _ = vm.run(32);
    vm.host
}

pub fn scenario_14(speaker: &str, msg_key: &str, layer: &str) -> Vs2MiniVm {
    let mut host = Vs2Host::new();
    host.pool = vec![speaker.into(), msg_key.into(), layer.into()];
    host.set_translation(msg_key, format!("line-14"));
    let mut vm = Vs2MiniVm::new(host);
    vm.load(vec![
        (OpVs2::LoadMsg, 1, 0),
        (OpVs2::Say, 0, 0),
        (OpVs2::PushLayer, 2, 0),
        (OpVs2::Ret, 0, 0),
    ]);
    vm
}

pub fn run_scenario_14() -> Vs2Host {
    let mut vm = scenario_14("hero", "k14", "hud");
    let _ = vm.run(32);
    vm.host
}

pub fn scenario_15(speaker: &str, msg_key: &str, layer: &str) -> Vs2MiniVm {
    let mut host = Vs2Host::new();
    host.pool = vec![speaker.into(), msg_key.into(), layer.into()];
    host.set_translation(msg_key, format!("line-15"));
    let mut vm = Vs2MiniVm::new(host);
    vm.load(vec![
        (OpVs2::LoadMsg, 1, 0),
        (OpVs2::Say, 0, 0),
        (OpVs2::PushLayer, 2, 0),
        (OpVs2::Ret, 0, 0),
    ]);
    vm
}

pub fn run_scenario_15() -> Vs2Host {
    let mut vm = scenario_15("hero", "k15", "hud");
    let _ = vm.run(32);
    vm.host
}

pub fn scenario_16(speaker: &str, msg_key: &str, layer: &str) -> Vs2MiniVm {
    let mut host = Vs2Host::new();
    host.pool = vec![speaker.into(), msg_key.into(), layer.into()];
    host.set_translation(msg_key, format!("line-16"));
    let mut vm = Vs2MiniVm::new(host);
    vm.load(vec![
        (OpVs2::LoadMsg, 1, 0),
        (OpVs2::Say, 0, 0),
        (OpVs2::PushLayer, 2, 0),
        (OpVs2::Ret, 0, 0),
    ]);
    vm
}

pub fn run_scenario_16() -> Vs2Host {
    let mut vm = scenario_16("hero", "k16", "hud");
    let _ = vm.run(32);
    vm.host
}

pub fn scenario_17(speaker: &str, msg_key: &str, layer: &str) -> Vs2MiniVm {
    let mut host = Vs2Host::new();
    host.pool = vec![speaker.into(), msg_key.into(), layer.into()];
    host.set_translation(msg_key, format!("line-17"));
    let mut vm = Vs2MiniVm::new(host);
    vm.load(vec![
        (OpVs2::LoadMsg, 1, 0),
        (OpVs2::Say, 0, 0),
        (OpVs2::PushLayer, 2, 0),
        (OpVs2::Ret, 0, 0),
    ]);
    vm
}

pub fn run_scenario_17() -> Vs2Host {
    let mut vm = scenario_17("hero", "k17", "hud");
    let _ = vm.run(32);
    vm.host
}

pub fn scenario_18(speaker: &str, msg_key: &str, layer: &str) -> Vs2MiniVm {
    let mut host = Vs2Host::new();
    host.pool = vec![speaker.into(), msg_key.into(), layer.into()];
    host.set_translation(msg_key, format!("line-18"));
    let mut vm = Vs2MiniVm::new(host);
    vm.load(vec![
        (OpVs2::LoadMsg, 1, 0),
        (OpVs2::Say, 0, 0),
        (OpVs2::PushLayer, 2, 0),
        (OpVs2::Ret, 0, 0),
    ]);
    vm
}

pub fn run_scenario_18() -> Vs2Host {
    let mut vm = scenario_18("hero", "k18", "hud");
    let _ = vm.run(32);
    vm.host
}

pub fn scenario_19(speaker: &str, msg_key: &str, layer: &str) -> Vs2MiniVm {
    let mut host = Vs2Host::new();
    host.pool = vec![speaker.into(), msg_key.into(), layer.into()];
    host.set_translation(msg_key, format!("line-19"));
    let mut vm = Vs2MiniVm::new(host);
    vm.load(vec![
        (OpVs2::LoadMsg, 1, 0),
        (OpVs2::Say, 0, 0),
        (OpVs2::PushLayer, 2, 0),
        (OpVs2::Ret, 0, 0),
    ]);
    vm
}

pub fn run_scenario_19() -> Vs2Host {
    let mut vm = scenario_19("hero", "k19", "hud");
    let _ = vm.run(32);
    vm.host
}

pub fn scenario_20(speaker: &str, msg_key: &str, layer: &str) -> Vs2MiniVm {
    let mut host = Vs2Host::new();
    host.pool = vec![speaker.into(), msg_key.into(), layer.into()];
    host.set_translation(msg_key, format!("line-20"));
    let mut vm = Vs2MiniVm::new(host);
    vm.load(vec![
        (OpVs2::LoadMsg, 1, 0),
        (OpVs2::Say, 0, 0),
        (OpVs2::PushLayer, 2, 0),
        (OpVs2::Ret, 0, 0),
    ]);
    vm
}

pub fn run_scenario_20() -> Vs2Host {
    let mut vm = scenario_20("hero", "k20", "hud");
    let _ = vm.run(32);
    vm.host
}

pub fn scenario_21(speaker: &str, msg_key: &str, layer: &str) -> Vs2MiniVm {
    let mut host = Vs2Host::new();
    host.pool = vec![speaker.into(), msg_key.into(), layer.into()];
    host.set_translation(msg_key, format!("line-21"));
    let mut vm = Vs2MiniVm::new(host);
    vm.load(vec![
        (OpVs2::LoadMsg, 1, 0),
        (OpVs2::Say, 0, 0),
        (OpVs2::PushLayer, 2, 0),
        (OpVs2::Ret, 0, 0),
    ]);
    vm
}

pub fn run_scenario_21() -> Vs2Host {
    let mut vm = scenario_21("hero", "k21", "hud");
    let _ = vm.run(32);
    vm.host
}

pub fn scenario_22(speaker: &str, msg_key: &str, layer: &str) -> Vs2MiniVm {
    let mut host = Vs2Host::new();
    host.pool = vec![speaker.into(), msg_key.into(), layer.into()];
    host.set_translation(msg_key, format!("line-22"));
    let mut vm = Vs2MiniVm::new(host);
    vm.load(vec![
        (OpVs2::LoadMsg, 1, 0),
        (OpVs2::Say, 0, 0),
        (OpVs2::PushLayer, 2, 0),
        (OpVs2::Ret, 0, 0),
    ]);
    vm
}

pub fn run_scenario_22() -> Vs2Host {
    let mut vm = scenario_22("hero", "k22", "hud");
    let _ = vm.run(32);
    vm.host
}

pub fn scenario_23(speaker: &str, msg_key: &str, layer: &str) -> Vs2MiniVm {
    let mut host = Vs2Host::new();
    host.pool = vec![speaker.into(), msg_key.into(), layer.into()];
    host.set_translation(msg_key, format!("line-23"));
    let mut vm = Vs2MiniVm::new(host);
    vm.load(vec![
        (OpVs2::LoadMsg, 1, 0),
        (OpVs2::Say, 0, 0),
        (OpVs2::PushLayer, 2, 0),
        (OpVs2::Ret, 0, 0),
    ]);
    vm
}

pub fn run_scenario_23() -> Vs2Host {
    let mut vm = scenario_23("hero", "k23", "hud");
    let _ = vm.run(32);
    vm.host
}

pub fn scenario_24(speaker: &str, msg_key: &str, layer: &str) -> Vs2MiniVm {
    let mut host = Vs2Host::new();
    host.pool = vec![speaker.into(), msg_key.into(), layer.into()];
    host.set_translation(msg_key, format!("line-24"));
    let mut vm = Vs2MiniVm::new(host);
    vm.load(vec![
        (OpVs2::LoadMsg, 1, 0),
        (OpVs2::Say, 0, 0),
        (OpVs2::PushLayer, 2, 0),
        (OpVs2::Ret, 0, 0),
    ]);
    vm
}

pub fn run_scenario_24() -> Vs2Host {
    let mut vm = scenario_24("hero", "k24", "hud");
    let _ = vm.run(32);
    vm.host
}

pub fn scenario_25(speaker: &str, msg_key: &str, layer: &str) -> Vs2MiniVm {
    let mut host = Vs2Host::new();
    host.pool = vec![speaker.into(), msg_key.into(), layer.into()];
    host.set_translation(msg_key, format!("line-25"));
    let mut vm = Vs2MiniVm::new(host);
    vm.load(vec![
        (OpVs2::LoadMsg, 1, 0),
        (OpVs2::Say, 0, 0),
        (OpVs2::PushLayer, 2, 0),
        (OpVs2::Ret, 0, 0),
    ]);
    vm
}

pub fn run_scenario_25() -> Vs2Host {
    let mut vm = scenario_25("hero", "k25", "hud");
    let _ = vm.run(32);
    vm.host
}

pub fn scenario_26(speaker: &str, msg_key: &str, layer: &str) -> Vs2MiniVm {
    let mut host = Vs2Host::new();
    host.pool = vec![speaker.into(), msg_key.into(), layer.into()];
    host.set_translation(msg_key, format!("line-26"));
    let mut vm = Vs2MiniVm::new(host);
    vm.load(vec![
        (OpVs2::LoadMsg, 1, 0),
        (OpVs2::Say, 0, 0),
        (OpVs2::PushLayer, 2, 0),
        (OpVs2::Ret, 0, 0),
    ]);
    vm
}

pub fn run_scenario_26() -> Vs2Host {
    let mut vm = scenario_26("hero", "k26", "hud");
    let _ = vm.run(32);
    vm.host
}

pub fn scenario_27(speaker: &str, msg_key: &str, layer: &str) -> Vs2MiniVm {
    let mut host = Vs2Host::new();
    host.pool = vec![speaker.into(), msg_key.into(), layer.into()];
    host.set_translation(msg_key, format!("line-27"));
    let mut vm = Vs2MiniVm::new(host);
    vm.load(vec![
        (OpVs2::LoadMsg, 1, 0),
        (OpVs2::Say, 0, 0),
        (OpVs2::PushLayer, 2, 0),
        (OpVs2::Ret, 0, 0),
    ]);
    vm
}

pub fn run_scenario_27() -> Vs2Host {
    let mut vm = scenario_27("hero", "k27", "hud");
    let _ = vm.run(32);
    vm.host
}

pub fn scenario_28(speaker: &str, msg_key: &str, layer: &str) -> Vs2MiniVm {
    let mut host = Vs2Host::new();
    host.pool = vec![speaker.into(), msg_key.into(), layer.into()];
    host.set_translation(msg_key, format!("line-28"));
    let mut vm = Vs2MiniVm::new(host);
    vm.load(vec![
        (OpVs2::LoadMsg, 1, 0),
        (OpVs2::Say, 0, 0),
        (OpVs2::PushLayer, 2, 0),
        (OpVs2::Ret, 0, 0),
    ]);
    vm
}

pub fn run_scenario_28() -> Vs2Host {
    let mut vm = scenario_28("hero", "k28", "hud");
    let _ = vm.run(32);
    vm.host
}

pub fn scenario_29(speaker: &str, msg_key: &str, layer: &str) -> Vs2MiniVm {
    let mut host = Vs2Host::new();
    host.pool = vec![speaker.into(), msg_key.into(), layer.into()];
    host.set_translation(msg_key, format!("line-29"));
    let mut vm = Vs2MiniVm::new(host);
    vm.load(vec![
        (OpVs2::LoadMsg, 1, 0),
        (OpVs2::Say, 0, 0),
        (OpVs2::PushLayer, 2, 0),
        (OpVs2::Ret, 0, 0),
    ]);
    vm
}

pub fn run_scenario_29() -> Vs2Host {
    let mut vm = scenario_29("hero", "k29", "hud");
    let _ = vm.run(32);
    vm.host
}

pub fn scenario_30(speaker: &str, msg_key: &str, layer: &str) -> Vs2MiniVm {
    let mut host = Vs2Host::new();
    host.pool = vec![speaker.into(), msg_key.into(), layer.into()];
    host.set_translation(msg_key, format!("line-30"));
    let mut vm = Vs2MiniVm::new(host);
    vm.load(vec![
        (OpVs2::LoadMsg, 1, 0),
        (OpVs2::Say, 0, 0),
        (OpVs2::PushLayer, 2, 0),
        (OpVs2::Ret, 0, 0),
    ]);
    vm
}

pub fn run_scenario_30() -> Vs2Host {
    let mut vm = scenario_30("hero", "k30", "hud");
    let _ = vm.run(32);
    vm.host
}

pub fn scenario_31(speaker: &str, msg_key: &str, layer: &str) -> Vs2MiniVm {
    let mut host = Vs2Host::new();
    host.pool = vec![speaker.into(), msg_key.into(), layer.into()];
    host.set_translation(msg_key, format!("line-31"));
    let mut vm = Vs2MiniVm::new(host);
    vm.load(vec![
        (OpVs2::LoadMsg, 1, 0),
        (OpVs2::Say, 0, 0),
        (OpVs2::PushLayer, 2, 0),
        (OpVs2::Ret, 0, 0),
    ]);
    vm
}

pub fn run_scenario_31() -> Vs2Host {
    let mut vm = scenario_31("hero", "k31", "hud");
    let _ = vm.run(32);
    vm.host
}

pub fn scenario_32(speaker: &str, msg_key: &str, layer: &str) -> Vs2MiniVm {
    let mut host = Vs2Host::new();
    host.pool = vec![speaker.into(), msg_key.into(), layer.into()];
    host.set_translation(msg_key, format!("line-32"));
    let mut vm = Vs2MiniVm::new(host);
    vm.load(vec![
        (OpVs2::LoadMsg, 1, 0),
        (OpVs2::Say, 0, 0),
        (OpVs2::PushLayer, 2, 0),
        (OpVs2::Ret, 0, 0),
    ]);
    vm
}

pub fn run_scenario_32() -> Vs2Host {
    let mut vm = scenario_32("hero", "k32", "hud");
    let _ = vm.run(32);
    vm.host
}

pub fn scenario_33(speaker: &str, msg_key: &str, layer: &str) -> Vs2MiniVm {
    let mut host = Vs2Host::new();
    host.pool = vec![speaker.into(), msg_key.into(), layer.into()];
    host.set_translation(msg_key, format!("line-33"));
    let mut vm = Vs2MiniVm::new(host);
    vm.load(vec![
        (OpVs2::LoadMsg, 1, 0),
        (OpVs2::Say, 0, 0),
        (OpVs2::PushLayer, 2, 0),
        (OpVs2::Ret, 0, 0),
    ]);
    vm
}

pub fn run_scenario_33() -> Vs2Host {
    let mut vm = scenario_33("hero", "k33", "hud");
    let _ = vm.run(32);
    vm.host
}

pub fn scenario_34(speaker: &str, msg_key: &str, layer: &str) -> Vs2MiniVm {
    let mut host = Vs2Host::new();
    host.pool = vec![speaker.into(), msg_key.into(), layer.into()];
    host.set_translation(msg_key, format!("line-34"));
    let mut vm = Vs2MiniVm::new(host);
    vm.load(vec![
        (OpVs2::LoadMsg, 1, 0),
        (OpVs2::Say, 0, 0),
        (OpVs2::PushLayer, 2, 0),
        (OpVs2::Ret, 0, 0),
    ]);
    vm
}

pub fn run_scenario_34() -> Vs2Host {
    let mut vm = scenario_34("hero", "k34", "hud");
    let _ = vm.run(32);
    vm.host
}

pub fn scenario_35(speaker: &str, msg_key: &str, layer: &str) -> Vs2MiniVm {
    let mut host = Vs2Host::new();
    host.pool = vec![speaker.into(), msg_key.into(), layer.into()];
    host.set_translation(msg_key, format!("line-35"));
    let mut vm = Vs2MiniVm::new(host);
    vm.load(vec![
        (OpVs2::LoadMsg, 1, 0),
        (OpVs2::Say, 0, 0),
        (OpVs2::PushLayer, 2, 0),
        (OpVs2::Ret, 0, 0),
    ]);
    vm
}

pub fn run_scenario_35() -> Vs2Host {
    let mut vm = scenario_35("hero", "k35", "hud");
    let _ = vm.run(32);
    vm.host
}

pub fn scenario_36(speaker: &str, msg_key: &str, layer: &str) -> Vs2MiniVm {
    let mut host = Vs2Host::new();
    host.pool = vec![speaker.into(), msg_key.into(), layer.into()];
    host.set_translation(msg_key, format!("line-36"));
    let mut vm = Vs2MiniVm::new(host);
    vm.load(vec![
        (OpVs2::LoadMsg, 1, 0),
        (OpVs2::Say, 0, 0),
        (OpVs2::PushLayer, 2, 0),
        (OpVs2::Ret, 0, 0),
    ]);
    vm
}

pub fn run_scenario_36() -> Vs2Host {
    let mut vm = scenario_36("hero", "k36", "hud");
    let _ = vm.run(32);
    vm.host
}

pub fn scenario_37(speaker: &str, msg_key: &str, layer: &str) -> Vs2MiniVm {
    let mut host = Vs2Host::new();
    host.pool = vec![speaker.into(), msg_key.into(), layer.into()];
    host.set_translation(msg_key, format!("line-37"));
    let mut vm = Vs2MiniVm::new(host);
    vm.load(vec![
        (OpVs2::LoadMsg, 1, 0),
        (OpVs2::Say, 0, 0),
        (OpVs2::PushLayer, 2, 0),
        (OpVs2::Ret, 0, 0),
    ]);
    vm
}

pub fn run_scenario_37() -> Vs2Host {
    let mut vm = scenario_37("hero", "k37", "hud");
    let _ = vm.run(32);
    vm.host
}

pub fn scenario_38(speaker: &str, msg_key: &str, layer: &str) -> Vs2MiniVm {
    let mut host = Vs2Host::new();
    host.pool = vec![speaker.into(), msg_key.into(), layer.into()];
    host.set_translation(msg_key, format!("line-38"));
    let mut vm = Vs2MiniVm::new(host);
    vm.load(vec![
        (OpVs2::LoadMsg, 1, 0),
        (OpVs2::Say, 0, 0),
        (OpVs2::PushLayer, 2, 0),
        (OpVs2::Ret, 0, 0),
    ]);
    vm
}

pub fn run_scenario_38() -> Vs2Host {
    let mut vm = scenario_38("hero", "k38", "hud");
    let _ = vm.run(32);
    vm.host
}

pub fn scenario_39(speaker: &str, msg_key: &str, layer: &str) -> Vs2MiniVm {
    let mut host = Vs2Host::new();
    host.pool = vec![speaker.into(), msg_key.into(), layer.into()];
    host.set_translation(msg_key, format!("line-39"));
    let mut vm = Vs2MiniVm::new(host);
    vm.load(vec![
        (OpVs2::LoadMsg, 1, 0),
        (OpVs2::Say, 0, 0),
        (OpVs2::PushLayer, 2, 0),
        (OpVs2::Ret, 0, 0),
    ]);
    vm
}

pub fn run_scenario_39() -> Vs2Host {
    let mut vm = scenario_39("hero", "k39", "hud");
    let _ = vm.run(32);
    vm.host
}

pub fn scenario_40(speaker: &str, msg_key: &str, layer: &str) -> Vs2MiniVm {
    let mut host = Vs2Host::new();
    host.pool = vec![speaker.into(), msg_key.into(), layer.into()];
    host.set_translation(msg_key, format!("line-40"));
    let mut vm = Vs2MiniVm::new(host);
    vm.load(vec![
        (OpVs2::LoadMsg, 1, 0),
        (OpVs2::Say, 0, 0),
        (OpVs2::PushLayer, 2, 0),
        (OpVs2::Ret, 0, 0),
    ]);
    vm
}

pub fn run_scenario_40() -> Vs2Host {
    let mut vm = scenario_40("hero", "k40", "hud");
    let _ = vm.run(32);
    vm.host
}

pub fn scenario_41(speaker: &str, msg_key: &str, layer: &str) -> Vs2MiniVm {
    let mut host = Vs2Host::new();
    host.pool = vec![speaker.into(), msg_key.into(), layer.into()];
    host.set_translation(msg_key, format!("line-41"));
    let mut vm = Vs2MiniVm::new(host);
    vm.load(vec![
        (OpVs2::LoadMsg, 1, 0),
        (OpVs2::Say, 0, 0),
        (OpVs2::PushLayer, 2, 0),
        (OpVs2::Ret, 0, 0),
    ]);
    vm
}

pub fn run_scenario_41() -> Vs2Host {
    let mut vm = scenario_41("hero", "k41", "hud");
    let _ = vm.run(32);
    vm.host
}

pub fn scenario_42(speaker: &str, msg_key: &str, layer: &str) -> Vs2MiniVm {
    let mut host = Vs2Host::new();
    host.pool = vec![speaker.into(), msg_key.into(), layer.into()];
    host.set_translation(msg_key, format!("line-42"));
    let mut vm = Vs2MiniVm::new(host);
    vm.load(vec![
        (OpVs2::LoadMsg, 1, 0),
        (OpVs2::Say, 0, 0),
        (OpVs2::PushLayer, 2, 0),
        (OpVs2::Ret, 0, 0),
    ]);
    vm
}

pub fn run_scenario_42() -> Vs2Host {
    let mut vm = scenario_42("hero", "k42", "hud");
    let _ = vm.run(32);
    vm.host
}

pub fn scenario_43(speaker: &str, msg_key: &str, layer: &str) -> Vs2MiniVm {
    let mut host = Vs2Host::new();
    host.pool = vec![speaker.into(), msg_key.into(), layer.into()];
    host.set_translation(msg_key, format!("line-43"));
    let mut vm = Vs2MiniVm::new(host);
    vm.load(vec![
        (OpVs2::LoadMsg, 1, 0),
        (OpVs2::Say, 0, 0),
        (OpVs2::PushLayer, 2, 0),
        (OpVs2::Ret, 0, 0),
    ]);
    vm
}

pub fn run_scenario_43() -> Vs2Host {
    let mut vm = scenario_43("hero", "k43", "hud");
    let _ = vm.run(32);
    vm.host
}

pub fn scenario_44(speaker: &str, msg_key: &str, layer: &str) -> Vs2MiniVm {
    let mut host = Vs2Host::new();
    host.pool = vec![speaker.into(), msg_key.into(), layer.into()];
    host.set_translation(msg_key, format!("line-44"));
    let mut vm = Vs2MiniVm::new(host);
    vm.load(vec![
        (OpVs2::LoadMsg, 1, 0),
        (OpVs2::Say, 0, 0),
        (OpVs2::PushLayer, 2, 0),
        (OpVs2::Ret, 0, 0),
    ]);
    vm
}

pub fn run_scenario_44() -> Vs2Host {
    let mut vm = scenario_44("hero", "k44", "hud");
    let _ = vm.run(32);
    vm.host
}

pub fn scenario_45(speaker: &str, msg_key: &str, layer: &str) -> Vs2MiniVm {
    let mut host = Vs2Host::new();
    host.pool = vec![speaker.into(), msg_key.into(), layer.into()];
    host.set_translation(msg_key, format!("line-45"));
    let mut vm = Vs2MiniVm::new(host);
    vm.load(vec![
        (OpVs2::LoadMsg, 1, 0),
        (OpVs2::Say, 0, 0),
        (OpVs2::PushLayer, 2, 0),
        (OpVs2::Ret, 0, 0),
    ]);
    vm
}

pub fn run_scenario_45() -> Vs2Host {
    let mut vm = scenario_45("hero", "k45", "hud");
    let _ = vm.run(32);
    vm.host
}

pub fn scenario_46(speaker: &str, msg_key: &str, layer: &str) -> Vs2MiniVm {
    let mut host = Vs2Host::new();
    host.pool = vec![speaker.into(), msg_key.into(), layer.into()];
    host.set_translation(msg_key, format!("line-46"));
    let mut vm = Vs2MiniVm::new(host);
    vm.load(vec![
        (OpVs2::LoadMsg, 1, 0),
        (OpVs2::Say, 0, 0),
        (OpVs2::PushLayer, 2, 0),
        (OpVs2::Ret, 0, 0),
    ]);
    vm
}

pub fn run_scenario_46() -> Vs2Host {
    let mut vm = scenario_46("hero", "k46", "hud");
    let _ = vm.run(32);
    vm.host
}

pub fn scenario_47(speaker: &str, msg_key: &str, layer: &str) -> Vs2MiniVm {
    let mut host = Vs2Host::new();
    host.pool = vec![speaker.into(), msg_key.into(), layer.into()];
    host.set_translation(msg_key, format!("line-47"));
    let mut vm = Vs2MiniVm::new(host);
    vm.load(vec![
        (OpVs2::LoadMsg, 1, 0),
        (OpVs2::Say, 0, 0),
        (OpVs2::PushLayer, 2, 0),
        (OpVs2::Ret, 0, 0),
    ]);
    vm
}

pub fn run_scenario_47() -> Vs2Host {
    let mut vm = scenario_47("hero", "k47", "hud");
    let _ = vm.run(32);
    vm.host
}

pub fn scenario_48(speaker: &str, msg_key: &str, layer: &str) -> Vs2MiniVm {
    let mut host = Vs2Host::new();
    host.pool = vec![speaker.into(), msg_key.into(), layer.into()];
    host.set_translation(msg_key, format!("line-48"));
    let mut vm = Vs2MiniVm::new(host);
    vm.load(vec![
        (OpVs2::LoadMsg, 1, 0),
        (OpVs2::Say, 0, 0),
        (OpVs2::PushLayer, 2, 0),
        (OpVs2::Ret, 0, 0),
    ]);
    vm
}

pub fn run_scenario_48() -> Vs2Host {
    let mut vm = scenario_48("hero", "k48", "hud");
    let _ = vm.run(32);
    vm.host
}

pub fn scenario_49(speaker: &str, msg_key: &str, layer: &str) -> Vs2MiniVm {
    let mut host = Vs2Host::new();
    host.pool = vec![speaker.into(), msg_key.into(), layer.into()];
    host.set_translation(msg_key, format!("line-49"));
    let mut vm = Vs2MiniVm::new(host);
    vm.load(vec![
        (OpVs2::LoadMsg, 1, 0),
        (OpVs2::Say, 0, 0),
        (OpVs2::PushLayer, 2, 0),
        (OpVs2::Ret, 0, 0),
    ]);
    vm
}

pub fn run_scenario_49() -> Vs2Host {
    let mut vm = scenario_49("hero", "k49", "hud");
    let _ = vm.run(32);
    vm.host
}

pub fn scenario_50(speaker: &str, msg_key: &str, layer: &str) -> Vs2MiniVm {
    let mut host = Vs2Host::new();
    host.pool = vec![speaker.into(), msg_key.into(), layer.into()];
    host.set_translation(msg_key, format!("line-50"));
    let mut vm = Vs2MiniVm::new(host);
    vm.load(vec![
        (OpVs2::LoadMsg, 1, 0),
        (OpVs2::Say, 0, 0),
        (OpVs2::PushLayer, 2, 0),
        (OpVs2::Ret, 0, 0),
    ]);
    vm
}

pub fn run_scenario_50() -> Vs2Host {
    let mut vm = scenario_50("hero", "k50", "hud");
    let _ = vm.run(32);
    vm.host
}

pub fn scenario_51(speaker: &str, msg_key: &str, layer: &str) -> Vs2MiniVm {
    let mut host = Vs2Host::new();
    host.pool = vec![speaker.into(), msg_key.into(), layer.into()];
    host.set_translation(msg_key, format!("line-51"));
    let mut vm = Vs2MiniVm::new(host);
    vm.load(vec![
        (OpVs2::LoadMsg, 1, 0),
        (OpVs2::Say, 0, 0),
        (OpVs2::PushLayer, 2, 0),
        (OpVs2::Ret, 0, 0),
    ]);
    vm
}

pub fn run_scenario_51() -> Vs2Host {
    let mut vm = scenario_51("hero", "k51", "hud");
    let _ = vm.run(32);
    vm.host
}

pub fn scenario_52(speaker: &str, msg_key: &str, layer: &str) -> Vs2MiniVm {
    let mut host = Vs2Host::new();
    host.pool = vec![speaker.into(), msg_key.into(), layer.into()];
    host.set_translation(msg_key, format!("line-52"));
    let mut vm = Vs2MiniVm::new(host);
    vm.load(vec![
        (OpVs2::LoadMsg, 1, 0),
        (OpVs2::Say, 0, 0),
        (OpVs2::PushLayer, 2, 0),
        (OpVs2::Ret, 0, 0),
    ]);
    vm
}

pub fn run_scenario_52() -> Vs2Host {
    let mut vm = scenario_52("hero", "k52", "hud");
    let _ = vm.run(32);
    vm.host
}

pub fn scenario_53(speaker: &str, msg_key: &str, layer: &str) -> Vs2MiniVm {
    let mut host = Vs2Host::new();
    host.pool = vec![speaker.into(), msg_key.into(), layer.into()];
    host.set_translation(msg_key, format!("line-53"));
    let mut vm = Vs2MiniVm::new(host);
    vm.load(vec![
        (OpVs2::LoadMsg, 1, 0),
        (OpVs2::Say, 0, 0),
        (OpVs2::PushLayer, 2, 0),
        (OpVs2::Ret, 0, 0),
    ]);
    vm
}

pub fn run_scenario_53() -> Vs2Host {
    let mut vm = scenario_53("hero", "k53", "hud");
    let _ = vm.run(32);
    vm.host
}

pub fn scenario_54(speaker: &str, msg_key: &str, layer: &str) -> Vs2MiniVm {
    let mut host = Vs2Host::new();
    host.pool = vec![speaker.into(), msg_key.into(), layer.into()];
    host.set_translation(msg_key, format!("line-54"));
    let mut vm = Vs2MiniVm::new(host);
    vm.load(vec![
        (OpVs2::LoadMsg, 1, 0),
        (OpVs2::Say, 0, 0),
        (OpVs2::PushLayer, 2, 0),
        (OpVs2::Ret, 0, 0),
    ]);
    vm
}

pub fn run_scenario_54() -> Vs2Host {
    let mut vm = scenario_54("hero", "k54", "hud");
    let _ = vm.run(32);
    vm.host
}

pub fn scenario_55(speaker: &str, msg_key: &str, layer: &str) -> Vs2MiniVm {
    let mut host = Vs2Host::new();
    host.pool = vec![speaker.into(), msg_key.into(), layer.into()];
    host.set_translation(msg_key, format!("line-55"));
    let mut vm = Vs2MiniVm::new(host);
    vm.load(vec![
        (OpVs2::LoadMsg, 1, 0),
        (OpVs2::Say, 0, 0),
        (OpVs2::PushLayer, 2, 0),
        (OpVs2::Ret, 0, 0),
    ]);
    vm
}

pub fn run_scenario_55() -> Vs2Host {
    let mut vm = scenario_55("hero", "k55", "hud");
    let _ = vm.run(32);
    vm.host
}

pub fn scenario_56(speaker: &str, msg_key: &str, layer: &str) -> Vs2MiniVm {
    let mut host = Vs2Host::new();
    host.pool = vec![speaker.into(), msg_key.into(), layer.into()];
    host.set_translation(msg_key, format!("line-56"));
    let mut vm = Vs2MiniVm::new(host);
    vm.load(vec![
        (OpVs2::LoadMsg, 1, 0),
        (OpVs2::Say, 0, 0),
        (OpVs2::PushLayer, 2, 0),
        (OpVs2::Ret, 0, 0),
    ]);
    vm
}

pub fn run_scenario_56() -> Vs2Host {
    let mut vm = scenario_56("hero", "k56", "hud");
    let _ = vm.run(32);
    vm.host
}

pub fn scenario_57(speaker: &str, msg_key: &str, layer: &str) -> Vs2MiniVm {
    let mut host = Vs2Host::new();
    host.pool = vec![speaker.into(), msg_key.into(), layer.into()];
    host.set_translation(msg_key, format!("line-57"));
    let mut vm = Vs2MiniVm::new(host);
    vm.load(vec![
        (OpVs2::LoadMsg, 1, 0),
        (OpVs2::Say, 0, 0),
        (OpVs2::PushLayer, 2, 0),
        (OpVs2::Ret, 0, 0),
    ]);
    vm
}

pub fn run_scenario_57() -> Vs2Host {
    let mut vm = scenario_57("hero", "k57", "hud");
    let _ = vm.run(32);
    vm.host
}

pub fn scenario_58(speaker: &str, msg_key: &str, layer: &str) -> Vs2MiniVm {
    let mut host = Vs2Host::new();
    host.pool = vec![speaker.into(), msg_key.into(), layer.into()];
    host.set_translation(msg_key, format!("line-58"));
    let mut vm = Vs2MiniVm::new(host);
    vm.load(vec![
        (OpVs2::LoadMsg, 1, 0),
        (OpVs2::Say, 0, 0),
        (OpVs2::PushLayer, 2, 0),
        (OpVs2::Ret, 0, 0),
    ]);
    vm
}

pub fn run_scenario_58() -> Vs2Host {
    let mut vm = scenario_58("hero", "k58", "hud");
    let _ = vm.run(32);
    vm.host
}

pub fn scenario_59(speaker: &str, msg_key: &str, layer: &str) -> Vs2MiniVm {
    let mut host = Vs2Host::new();
    host.pool = vec![speaker.into(), msg_key.into(), layer.into()];
    host.set_translation(msg_key, format!("line-59"));
    let mut vm = Vs2MiniVm::new(host);
    vm.load(vec![
        (OpVs2::LoadMsg, 1, 0),
        (OpVs2::Say, 0, 0),
        (OpVs2::PushLayer, 2, 0),
        (OpVs2::Ret, 0, 0),
    ]);
    vm
}

pub fn run_scenario_59() -> Vs2Host {
    let mut vm = scenario_59("hero", "k59", "hud");
    let _ = vm.run(32);
    vm.host
}

pub fn scenario_60(speaker: &str, msg_key: &str, layer: &str) -> Vs2MiniVm {
    let mut host = Vs2Host::new();
    host.pool = vec![speaker.into(), msg_key.into(), layer.into()];
    host.set_translation(msg_key, format!("line-60"));
    let mut vm = Vs2MiniVm::new(host);
    vm.load(vec![
        (OpVs2::LoadMsg, 1, 0),
        (OpVs2::Say, 0, 0),
        (OpVs2::PushLayer, 2, 0),
        (OpVs2::Ret, 0, 0),
    ]);
    vm
}

pub fn run_scenario_60() -> Vs2Host {
    let mut vm = scenario_60("hero", "k60", "hud");
    let _ = vm.run(32);
    vm.host
}

pub fn scenario_61(speaker: &str, msg_key: &str, layer: &str) -> Vs2MiniVm {
    let mut host = Vs2Host::new();
    host.pool = vec![speaker.into(), msg_key.into(), layer.into()];
    host.set_translation(msg_key, format!("line-61"));
    let mut vm = Vs2MiniVm::new(host);
    vm.load(vec![
        (OpVs2::LoadMsg, 1, 0),
        (OpVs2::Say, 0, 0),
        (OpVs2::PushLayer, 2, 0),
        (OpVs2::Ret, 0, 0),
    ]);
    vm
}

pub fn run_scenario_61() -> Vs2Host {
    let mut vm = scenario_61("hero", "k61", "hud");
    let _ = vm.run(32);
    vm.host
}

pub fn scenario_62(speaker: &str, msg_key: &str, layer: &str) -> Vs2MiniVm {
    let mut host = Vs2Host::new();
    host.pool = vec![speaker.into(), msg_key.into(), layer.into()];
    host.set_translation(msg_key, format!("line-62"));
    let mut vm = Vs2MiniVm::new(host);
    vm.load(vec![
        (OpVs2::LoadMsg, 1, 0),
        (OpVs2::Say, 0, 0),
        (OpVs2::PushLayer, 2, 0),
        (OpVs2::Ret, 0, 0),
    ]);
    vm
}

pub fn run_scenario_62() -> Vs2Host {
    let mut vm = scenario_62("hero", "k62", "hud");
    let _ = vm.run(32);
    vm.host
}

pub fn scenario_63(speaker: &str, msg_key: &str, layer: &str) -> Vs2MiniVm {
    let mut host = Vs2Host::new();
    host.pool = vec![speaker.into(), msg_key.into(), layer.into()];
    host.set_translation(msg_key, format!("line-63"));
    let mut vm = Vs2MiniVm::new(host);
    vm.load(vec![
        (OpVs2::LoadMsg, 1, 0),
        (OpVs2::Say, 0, 0),
        (OpVs2::PushLayer, 2, 0),
        (OpVs2::Ret, 0, 0),
    ]);
    vm
}

pub fn run_scenario_63() -> Vs2Host {
    let mut vm = scenario_63("hero", "k63", "hud");
    let _ = vm.run(32);
    vm.host
}

pub fn scenario_64(speaker: &str, msg_key: &str, layer: &str) -> Vs2MiniVm {
    let mut host = Vs2Host::new();
    host.pool = vec![speaker.into(), msg_key.into(), layer.into()];
    host.set_translation(msg_key, format!("line-64"));
    let mut vm = Vs2MiniVm::new(host);
    vm.load(vec![
        (OpVs2::LoadMsg, 1, 0),
        (OpVs2::Say, 0, 0),
        (OpVs2::PushLayer, 2, 0),
        (OpVs2::Ret, 0, 0),
    ]);
    vm
}

pub fn run_scenario_64() -> Vs2Host {
    let mut vm = scenario_64("hero", "k64", "hud");
    let _ = vm.run(32);
    vm.host
}

pub fn scenario_65(speaker: &str, msg_key: &str, layer: &str) -> Vs2MiniVm {
    let mut host = Vs2Host::new();
    host.pool = vec![speaker.into(), msg_key.into(), layer.into()];
    host.set_translation(msg_key, format!("line-65"));
    let mut vm = Vs2MiniVm::new(host);
    vm.load(vec![
        (OpVs2::LoadMsg, 1, 0),
        (OpVs2::Say, 0, 0),
        (OpVs2::PushLayer, 2, 0),
        (OpVs2::Ret, 0, 0),
    ]);
    vm
}

pub fn run_scenario_65() -> Vs2Host {
    let mut vm = scenario_65("hero", "k65", "hud");
    let _ = vm.run(32);
    vm.host
}

pub fn scenario_66(speaker: &str, msg_key: &str, layer: &str) -> Vs2MiniVm {
    let mut host = Vs2Host::new();
    host.pool = vec![speaker.into(), msg_key.into(), layer.into()];
    host.set_translation(msg_key, format!("line-66"));
    let mut vm = Vs2MiniVm::new(host);
    vm.load(vec![
        (OpVs2::LoadMsg, 1, 0),
        (OpVs2::Say, 0, 0),
        (OpVs2::PushLayer, 2, 0),
        (OpVs2::Ret, 0, 0),
    ]);
    vm
}

pub fn run_scenario_66() -> Vs2Host {
    let mut vm = scenario_66("hero", "k66", "hud");
    let _ = vm.run(32);
    vm.host
}

pub fn scenario_67(speaker: &str, msg_key: &str, layer: &str) -> Vs2MiniVm {
    let mut host = Vs2Host::new();
    host.pool = vec![speaker.into(), msg_key.into(), layer.into()];
    host.set_translation(msg_key, format!("line-67"));
    let mut vm = Vs2MiniVm::new(host);
    vm.load(vec![
        (OpVs2::LoadMsg, 1, 0),
        (OpVs2::Say, 0, 0),
        (OpVs2::PushLayer, 2, 0),
        (OpVs2::Ret, 0, 0),
    ]);
    vm
}

pub fn run_scenario_67() -> Vs2Host {
    let mut vm = scenario_67("hero", "k67", "hud");
    let _ = vm.run(32);
    vm.host
}

pub fn scenario_68(speaker: &str, msg_key: &str, layer: &str) -> Vs2MiniVm {
    let mut host = Vs2Host::new();
    host.pool = vec![speaker.into(), msg_key.into(), layer.into()];
    host.set_translation(msg_key, format!("line-68"));
    let mut vm = Vs2MiniVm::new(host);
    vm.load(vec![
        (OpVs2::LoadMsg, 1, 0),
        (OpVs2::Say, 0, 0),
        (OpVs2::PushLayer, 2, 0),
        (OpVs2::Ret, 0, 0),
    ]);
    vm
}

pub fn run_scenario_68() -> Vs2Host {
    let mut vm = scenario_68("hero", "k68", "hud");
    let _ = vm.run(32);
    vm.host
}

pub fn scenario_69(speaker: &str, msg_key: &str, layer: &str) -> Vs2MiniVm {
    let mut host = Vs2Host::new();
    host.pool = vec![speaker.into(), msg_key.into(), layer.into()];
    host.set_translation(msg_key, format!("line-69"));
    let mut vm = Vs2MiniVm::new(host);
    vm.load(vec![
        (OpVs2::LoadMsg, 1, 0),
        (OpVs2::Say, 0, 0),
        (OpVs2::PushLayer, 2, 0),
        (OpVs2::Ret, 0, 0),
    ]);
    vm
}

pub fn run_scenario_69() -> Vs2Host {
    let mut vm = scenario_69("hero", "k69", "hud");
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
    fn scenario_0_runs() {
        let h = run_scenario_0();
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

