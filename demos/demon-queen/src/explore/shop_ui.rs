//! La tienda de la villa y el local del burdel: selección, cantidades, pestañas, ratón y la compraventa que la cierra.
use super::shop_games::{self, Page};
use super::ui_sfx::{self, UiSfx};
use super::*;

impl App {
    pub(super) fn shop_product_indices(&self) -> Vec<usize> {
        shop::visible_indices(self.shop_tab, self.shop_category, &self.inventory)
    }

    pub(super) fn selected_shop_product_index(&self) -> Option<usize> {
        self.shop_product_indices()
            .get(self.shop_selection)
            .copied()
    }

    pub(super) fn open_shop(&mut self) {
        self.achievement_event("shop:open");
        self.map_open = false;
        self.shop_open = true;
        self.shop_tab = shop::Tab::Buy;
        self.shop_category = shop::Category::All;
        self.shop_selection = 0;
        self.shop_quantity = 1;
        self.shop_notice = None;
        self.shop_preview_rotation = None;
        self.shop_preview_dragging = false;
        self.shop_preview_drag_anchor = None;
        self.shop_page = Page::List;
        self.shop_opened_at = self.elapsed;
        self.shop_page_at = self.elapsed;
        self.haggle = None;
        self.shop_refused.clear();
        self.fuse = shop_games::Fuse::default();
        self.shop_said = Some((
            "—¡Bienvenida, Majestad! Mire sin prisa. Y si algo le parece caro… se puede hablar."
                .to_owned(),
            self.elapsed + 0.35,
        ));
        ui_sfx::play(UiSfx::Open);
        self.clear_input();
    }

    /// Rosalía's book takes the keys: each page its own.
    pub(super) fn venue_key(&mut self, code: KeyCode) {
        use venue::Page;
        let now = (self.elapsed * 1000.0) as u32;
        let page = self.venue.page;
        // Her book's pages turn with the menus' sounds: a tick to choose, a chime to take.
        match code {
            KeyCode::ArrowLeft
            | KeyCode::ArrowRight
            | KeyCode::ArrowUp
            | KeyCode::ArrowDown
            | KeyCode::KeyA
            | KeyCode::KeyD
            | KeyCode::KeyW
            | KeyCode::KeyS
            | KeyCode::Tab => ui_sfx::play(UiSfx::Move),
            KeyCode::Enter
            | KeyCode::NumpadEnter
            | KeyCode::Space
            | KeyCode::Digit1
            | KeyCode::Digit2
            | KeyCode::Digit3
            | KeyCode::Digit4 => ui_sfx::play(UiSfx::Select),
            KeyCode::KeyH | KeyCode::KeyE | KeyCode::KeyP => ui_sfx::play(UiSfx::Page),
            _ => {}
        }
        match code {
            KeyCode::Enter | KeyCode::NumpadEnter | KeyCode::Space => {
                self.venue.act_at(&mut self.gold, now);
                self.after_venue_act();
                self.persist_3d_state(true);
            }
            KeyCode::KeyF => self.close_shop(),
            KeyCode::KeyH if page == Page::Girls => self.venue.talk(),
            // Her elixir, from the first page, once she has offered it.
            KeyCode::KeyE if page == Page::Girls && self.venue.progress.elixir_offered => {
                self.venue.show(Page::Offer, now);
                self.venue.offer_from = Page::Girls;
            }
            // The girls stand side by side: left and right (and up and down) choose.
            KeyCode::ArrowLeft | KeyCode::KeyA | KeyCode::ArrowUp | KeyCode::KeyW
                if page == Page::Girls =>
            {
                self.venue.navigate(-1)
            }
            KeyCode::ArrowRight | KeyCode::KeyD | KeyCode::ArrowDown | KeyCode::KeyS
                if page == Page::Girls =>
            {
                self.venue.navigate(1)
            }
            KeyCode::ArrowUp | KeyCode::KeyW if page == Page::Ask => self.venue.navigate(-1),
            KeyCode::ArrowDown | KeyCode::KeyS if page == Page::Ask => self.venue.navigate(1),
            KeyCode::ArrowLeft | KeyCode::KeyA if page == Page::Ask => {
                self.venue.back(now);
            }
            KeyCode::Digit1 | KeyCode::Digit2 | KeyCode::Digit3 | KeyCode::Digit4
                if page == Page::Ask =>
            {
                let pick = match code {
                    KeyCode::Digit1 => 0,
                    KeyCode::Digit2 => 1,
                    KeyCode::Digit3 => 2,
                    _ => 3,
                };
                if pick < self.venue.asking().len() {
                    self.venue.choice = pick;
                    self.venue.act_at(&mut self.gold, now);
                    self.after_venue_act();
                    self.persist_3d_state(true);
                }
            }
            // The offer: buy, not now, and the leaflet with its side effects (P from anywhere on it).
            KeyCode::ArrowLeft | KeyCode::ArrowUp | KeyCode::KeyA | KeyCode::KeyW
                if page == Page::Offer =>
            {
                self.venue.navigate(-1)
            }
            KeyCode::ArrowRight
            | KeyCode::ArrowDown
            | KeyCode::KeyD
            | KeyCode::KeyS
            | KeyCode::Tab
                if page == Page::Offer =>
            {
                self.venue.navigate(1)
            }
            KeyCode::KeyP if page == Page::Offer => self.venue.open_leaflet(now),
            KeyCode::KeyP if page == Page::Leaflet => {
                self.venue.back(now);
            }
            _ => {}
        }
    }

    /// The book opens at Rosalía's desk (her offer first, the first time after a girl).
    pub(super) fn open_venue_book(&mut self) {
        self.open_shop();
        let now = (self.elapsed * 1000.0) as u32;
        self.venue.open_book(now);
        // The book is Rosalía's: the village shopkeeper says nothing here.
        self.shop_said = None;
        if self.venue.page == venue::Page::Offer && self.venue.progress.elixirs_bought == 0 {
            self.float_chiquita(
                "—¿«Mi reina»? ¿Lo sabe? …No. Se lo dice a todas. Creo.",
                false,
            );
        }
    }

    /// A girl was paid for: the book closes and she comes to take the Queen up to her room
    /// (her guion opens there). Without a way up, her guion opens at once. The elixir bought: she
    /// drinks it there and then.
    pub(super) fn after_venue_act(&mut self) {
        if self.venue.drink {
            self.venue.drink = false;
            self.start_drink();
            return;
        }
        let Some(id) = self.venue.scene.take() else {
            return;
        };
        // What was asked of her, for her guion: the seal's shape or not.
        let asked = self.venue.asked.take();
        self.story.values.insert(
            "cita_sello".to_owned(),
            i32::from(asked.as_deref() == Some("sello")),
        );
        // Alba's game, with the seal asleep: «Que ella mande»; Noa's: «Una noche a solas».
        self.story.values.insert(
            "cita_pide".to_owned(),
            i32::from(asked.as_deref() == Some("pide")),
        );
        self.story.values.insert(
            "cita_noche".to_owned(),
            i32::from(asked.as_deref() == Some("noche")),
        );
        // Nina's drink at the bar: no room, just a talk.
        self.story.values.insert(
            "cita_barra".to_owned(),
            i32::from(asked.as_deref() == Some("barra")),
        );
        self.close_shop();
        // A talk the first time: on the stage, where she is (the bar, the fire, the candles).
        let talk = matches!(asked.as_deref(), Some("charla" | "velas" | "barra"));
        if talk && self.story.get(&format!("{id}_vn")) == 0 {
            if self.start_talk(None, Some(&format!("burdel_charla_{id}"))) {
                return;
            }
        }
        let entry = venue::catalog().company.iter().find(|e| e.id == id);
        let room = entry.map_or(1, |e| e.room);
        let line = entry.map_or("—Sígame.", |e| e.escort_line.as_str());
        if let Some(inside) = self.brothel.as_mut() {
            if inside.start_escort(&id, room, line) {
                eprintln!("[escort] {id} comes for her (room {room})");
                self.queen_thought = None;
                self.clear_input();
                return;
            }
        }
        self.start_talk(None, Some(&format!("burdel_{id}")));
    }

    pub(super) fn close_shop(&mut self) {
        if self.shop_open && !self.venue.open {
            ui_sfx::play(UiSfx::Close);
        }
        self.haggle = None;
        self.shop_page = Page::List;
        self.shop_said = None;
        self.venue.open = false;
        self.shop_open = false;
        self.shop_selection = 0;
        self.shop_quantity = 1;
        self.shop_notice = None;
        self.shop_preview_rotation = None;
        self.shop_preview_dragging = false;
        self.shop_preview_drag_anchor = None;
        self.clear_input();
    }

    pub(super) fn move_shop_selection(&mut self, delta: isize) {
        if self.venue.open {
            self.venue.navigate(delta);
            return;
        }
        let count = self.shop_product_indices().len();
        if count == 0 {
            self.shop_selection = 0;
            return;
        }
        self.shop_selection =
            (self.shop_selection as isize + delta).rem_euclid(count as isize) as usize;
        self.shop_quantity = 1;
        self.shop_notice = None;
        ui_sfx::play(UiSfx::Move);
    }

    pub(super) fn toggle_shop_tab(&mut self) {
        if self.venue.open {
            // The book has one page: the house's girls.
            return;
        }
        // Buy, sell, the altar, and round again.
        match (self.shop_page, self.shop_tab) {
            (Page::Fuse, _) => self.show_shelves(shop::Tab::Buy),
            (_, shop::Tab::Buy) => self.show_shelves(shop::Tab::Sell),
            _ => self.open_fusion(),
        }
    }

    /// The shelves, on the given tab.
    pub(super) fn show_shelves(&mut self, tab: shop::Tab) {
        let changed = self.shop_page != Page::List || self.shop_tab != tab;
        self.haggle = None;
        self.set_shop_page(Page::List);
        if self.shop_tab != tab {
            self.shop_tab = tab;
            self.shop_selection = 0;
            self.shop_quantity = 1;
            self.shop_notice = None;
            self.shop_page_at = self.elapsed;
        }
        if changed {
            ui_sfx::play(UiSfx::Page);
        }
    }

    pub(super) fn adjust_shop_quantity(&mut self, delta: i32) {
        let current = self.shop_quantity as i32;
        let available = self
            .selected_shop_product_index()
            .map(|i| {
                let p = &shop::catalog().products[i];
                if self.shop_tab == shop::Tab::Sell {
                    self.inventory.get(&p.id).copied().unwrap_or(0)
                } else {
                    p.stock
                        .map(|s| self.shop_stock.get(&p.id).copied().unwrap_or(s))
                        .unwrap_or(99)
                }
            })
            .unwrap_or(1)
            .clamp(1, 99) as i32;
        self.shop_quantity = (current + delta).clamp(1, available) as u32;
        self.shop_notice = None;
        ui_sfx::play(if self.shop_quantity as i32 == current {
            UiSfx::Error
        } else {
            UiSfx::Tick
        });
    }

    pub(super) fn shop_pointer_position(&self) -> Option<(f32, f32)> {
        let (x, y) = self.cursor_position?;
        let size = self.window.as_ref()?.inner_size();
        // Invert the same viewport as the final compositor; reject black bars.
        velvet_render::SoftwareViewport::letterbox(
            size.width,
            size.height,
            WINDOW_WIDTH,
            WINDOW_HEIGHT,
        )?
        .physical_to_virtual(x as f64, y as f64)
        .map(|(x, y)| (x as f32, y as f32))
    }

    pub(super) fn begin_shop_preview_drag(&mut self) -> bool {
        let Some((x, y)) = self.shop_pointer_position() else {
            return false;
        };
        if self.shop_page != Page::List {
            return false;
        }
        let has_model = self
            .selected_shop_product_index()
            .and_then(|i| shop::catalog().products.get(i))
            .is_some_and(|product| product.preview_model.is_some());
        if !map_overlay::shop_preview_hit_test(x, y, has_model) {
            return false;
        }

        let idle_yaw = if std::env::var_os("VELVET_REDUCED_MOTION").is_some() {
            0.0
        } else {
            self.elapsed * 0.35
        };
        self.shop_preview_rotation.get_or_insert((idle_yaw, 0.24));
        self.shop_preview_dragging = true;
        self.shop_preview_drag_anchor = Some((x, y));
        true
    }

    pub(super) fn update_shop_preview_drag(&mut self) {
        if !self.shop_preview_dragging {
            return;
        }
        let Some(position) = self.shop_pointer_position() else {
            self.shop_preview_drag_anchor = None;
            return;
        };
        if let Some((last_x, last_y)) = self.shop_preview_drag_anchor {
            let (yaw, pitch) = self
                .shop_preview_rotation
                .unwrap_or((self.elapsed * 0.35, 0.24));
            let yaw = (yaw + (position.0 - last_x) * 0.01).rem_euclid(std::f32::consts::TAU);
            let pitch = (pitch + (position.1 - last_y) * 0.01).clamp(-1.45, 1.45);
            self.shop_preview_rotation = Some((yaw, pitch));
        }
        self.shop_preview_drag_anchor = Some(position);
    }

    pub(super) fn select_shop_category(&mut self, category: shop::Category) {
        if self.shop_category != category {
            ui_sfx::play(UiSfx::Page);
        }
        self.shop_category = category;
        self.shop_selection = 0;
        self.shop_quantity = 1;
        self.shop_notice = None;
    }

    pub(super) fn change_shop_page(&mut self, delta: isize) {
        if self.venue.open {
            return;
        }
        let before = self.shop_selection;
        self.shop_selection = shop::page_selection(
            self.shop_selection,
            self.shop_product_indices().len(),
            delta,
        );
        self.shop_quantity = 1;
        self.shop_notice = None;
        if self.shop_selection != before {
            ui_sfx::play(UiSfx::Page);
        }
    }

    pub(super) fn handle_shop_mouse_click(&mut self) {
        let Some((x, y)) = self.shop_pointer_position() else {
            return;
        };
        if self.venue.open {
            use map_overlay::venue_overlay::Hit;
            let now = (self.elapsed * 1000.0) as u32;
            ui_sfx::play(UiSfx::Select);
            match map_overlay::venue_overlay::hit(
                WINDOW_WIDTH as f32,
                WINDOW_HEIGHT as f32,
                x,
                y,
                &self.venue,
            ) {
                Some(Hit::Item(i)) => {
                    if i < self.venue.entries().len() {
                        // A second click on the chosen one goes on, as Enter.
                        if self.venue.selected == i {
                            self.venue.act_at(&mut self.gold, now);
                        } else {
                            self.venue.selected = i;
                            self.venue.notice.clear();
                        }
                    }
                }
                Some(Hit::Action) => {
                    self.venue.act_at(&mut self.gold, now);
                    self.after_venue_act();
                }
                Some(Hit::Ask(i)) => {
                    self.venue.choice = i;
                    self.venue.act_at(&mut self.gold, now);
                    self.after_venue_act();
                }
                Some(Hit::Buy) => {
                    self.venue.choice = 0;
                    self.venue.act_at(&mut self.gold, now);
                    self.after_venue_act();
                }
                Some(Hit::Decline) => {
                    self.venue.back(now);
                }
                Some(Hit::Leaflet) => self.venue.open_leaflet(now),
                Some(Hit::Elixir) => {
                    if !self.venue.progress.elixir_active {
                        self.venue.show(venue::Page::Offer, now);
                        self.venue.offer_from = venue::Page::Girls;
                    }
                }
                Some(Hit::Talk) => self.venue.talk(),
                Some(Hit::Close) => {
                    if !self.venue.back(now) {
                        self.close_shop();
                    }
                }
                None => {}
            }
            self.persist_3d_state(true);
            return;
        }
        let mut pick = map_overlay::shop_pick(
            self.shop_tab,
            self.shop_category,
            self.shop_selection,
            &self.inventory,
            match self.shop_page {
                Page::List => 0,
                Page::Haggle => 1,
                Page::Fuse => 2,
            },
            self.fuse_items().len(),
            self.fuse.cursor,
        );
        if self.haggle.as_ref().is_some_and(|h| h.offer.is_some()) {
            pick.favor = true;
            pick.favor_choices = play_scene::tienda::available(&self.story).0.len() as u8;
        }
        let Some(hit) = map_overlay::shop_hit_test(x, y, &pick) else {
            return;
        };

        match hit {
            map_overlay::ShopHit::TabBuy => {
                if self.shop_product_indices().is_empty() {
                    self.select_shop_category(shop::Category::All);
                }
                self.show_shelves(shop::Tab::Buy);
            }
            map_overlay::ShopHit::TabSell => self.show_shelves(shop::Tab::Sell),
            map_overlay::ShopHit::TabFuse => {
                if self.shop_page != Page::Fuse {
                    self.open_fusion();
                }
            }
            map_overlay::ShopHit::Item(index) => {
                if self.shop_selection == index {
                    // A second click on the lit one buys (or sells), as Enter.
                    self.commit_shop_transaction();
                } else {
                    self.shop_selection = index;
                    self.shop_quantity = 1;
                    self.shop_notice = None;
                    ui_sfx::play(UiSfx::Move);
                }
            }
            map_overlay::ShopHit::Haggle => self.start_haggle(),
            map_overlay::ShopHit::Tactic(i) => self.haggle_click(i as usize),
            map_overlay::ShopHit::FavorPrev => self.cycle_favor(-1),
            map_overlay::ShopHit::FavorNext => self.cycle_favor(1),
            map_overlay::ShopHit::FuseItem(i) => self.fuse_pick(i),
            map_overlay::ShopHit::FuseSlot(slot) => self.fuse_take(slot as usize),
            map_overlay::ShopHit::FuseGo => self.fuse_go(),
            map_overlay::ShopHit::FuseClear => self.fuse_clear(),
            map_overlay::ShopHit::QuantityMinus => self.adjust_shop_quantity(-1),
            map_overlay::ShopHit::Category(category) => self.select_shop_category(category),
            map_overlay::ShopHit::PagePrevious => self.change_shop_page(-1),
            map_overlay::ShopHit::PageNext => self.change_shop_page(1),
            map_overlay::ShopHit::QuantityPlus => self.adjust_shop_quantity(1),
            map_overlay::ShopHit::Action => self.commit_shop_transaction(),
            map_overlay::ShopHit::Close => self.close_shop(),
        }
    }

    pub(super) fn commit_shop_transaction(&mut self) {
        if !self.commit_shop_at(None) {
            ui_sfx::play(UiSfx::Error);
        }
    }

    /// Buys or sells the lit product, at `unit` each when a haggle set the price; whether it
    /// went through.
    /// Buys or sells the chosen quantity, at the shelf's price or for `total` (the whole lot,
    /// as a haggle left it).
    pub(super) fn commit_shop_at(&mut self, total: Option<u32>) -> bool {
        let Some(product_index) = self.selected_shop_product_index() else {
            self.shop_notice = Some("No tienes objetos para vender.".to_owned());
            return false;
        };
        let Some(product) = shop::catalog().products.get(product_index) else {
            return false;
        };
        let id = product.id.clone();
        let name = product.name.clone();
        let quantity = self.shop_quantity.max(1);
        let buy_cost = total.unwrap_or(product.buy_price.saturating_mul(quantity));
        let sell_reward = total.unwrap_or(product.sell_price.saturating_mul(quantity));
        match self.shop_tab {
            shop::Tab::Buy => {
                if let Some(stock) = product.stock {
                    let remaining = self.shop_stock.get(&id).copied().unwrap_or(stock);
                    if remaining < quantity {
                        self.shop_notice = Some(format!("Solo quedan {remaining} unidades."));
                        return false;
                    }
                }
                let cost = buy_cost;
                if self.gold < cost {
                    self.shop_notice = Some(format!("Te faltan {} de oro.", cost - self.gold));
                    return false;
                }
                self.gold -= cost;
                *self.inventory.entry(id.clone()).or_default() += quantity;
                self.achievements.record(&format!("shop:buy:{id}"));
                self.achievements.record("shop:buy");
                self.check_achievements();
                if product.stock.is_some() {
                    let remaining = self
                        .shop_stock
                        .entry(id)
                        .or_insert(product.stock.unwrap_or(0));
                    *remaining = remaining.saturating_sub(quantity);
                }
                self.shop_notice = Some(format!("Compraste {quantity} × {name}."));
                self.ui_sound(dungeon::sound::Sfx::Coins, 0.7);
                self.on_bought(&product.id);
                let mut pack = self.backpack.clone();
                pack.reconcile(&self.inventory);
                if pack.equip_if_empty(&product.id) {
                    self.backpack = pack;
                    self.apply_equipment();
                }
            }
            shop::Tab::Sell => {
                let owned = self.inventory.get(&id).copied().unwrap_or(0);
                if owned < quantity {
                    self.shop_notice = Some(format!("Solo tienes {owned} unidades."));
                    return false;
                }
                let reward = sell_reward;
                self.gold = self.gold.saturating_add(reward);
                if owned == quantity {
                    self.inventory.remove(&id);
                } else if let Some(amount) = self.inventory.get_mut(&id) {
                    *amount -= quantity;
                }
                self.shop_notice = Some(format!("Vendiste {quantity} × {name}."));
                self.ui_sound(dungeon::sound::Sfx::Coins, 0.7);
                if self.shop_product_indices().is_empty() {
                    self.shop_selection = 0;
                } else {
                    self.shop_selection = self
                        .shop_selection
                        .min(self.shop_product_indices().len() - 1);
                }
            }
        }
        self.persist_3d_state(true);
        true
    }
}
