//! Data-driven shop catalog and shared shop state types.
//!
//! The villa shop UI reads its products from `data/shops/villa_tienda.json`.
//! Adding or rebalancing an item therefore only changes data; the renderer and
//! transaction code do not need a new match arm.

use super::*;

pub(super) const PAGE_SIZE: usize = 8;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize, Hash)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Category {
    #[default]
    All,
    Consumables,
    Equipment,
    Materials,
    Special,
}

impl Category {
    pub const ALL: [Self; 5] = [
        Self::All,
        Self::Consumables,
        Self::Equipment,
        Self::Materials,
        Self::Special,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Self::All => "Todos",
            Self::Consumables => "Consumibles",
            Self::Equipment => "Equipamiento",
            Self::Materials => "Materiales",
            Self::Special => "Especiales",
        }
    }
    pub fn singular(self) -> &'static str {
        match self {
            Self::All => "Objeto",
            Self::Consumables => "Consumible",
            Self::Equipment => "Equipamiento",
            Self::Materials => "Material",
            Self::Special => "Especial",
        }
    }
}

/// Which counter the shop's window is: the villa's, or the Tendero's at the mine's mouth (picks
/// and bags for the shaft; he buys what comes out of it, for a little less than the villa).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum Counter {
    #[default]
    Villa,
    Bocamina,
}

thread_local! {
    static COUNTER: std::cell::Cell<Counter> = const { std::cell::Cell::new(Counter::Villa) };
}

/// The counter the window opens at (back to the villa's when it closes).
pub(super) fn set_counter(counter: Counter) {
    COUNTER.with(|c| c.set(counter));
}

pub(super) fn counter() -> Counter {
    COUNTER.with(|c| c.get())
}

/// What the counter asks for a product.
pub(super) fn buy_price(p: &Product) -> u32 {
    p.buy_price
}

/// What the counter pays for one: at the mine's mouth a little less than at the villa.
pub(super) fn sell_price(p: &Product) -> u32 {
    match counter() {
        Counter::Villa => p.sell_price,
        Counter::Bocamina => {
            ((p.sell_price as f32 * dungeon::pozo::MOUTH_RATE).round() as u32).max(1)
        }
    }
}

/// A pick or a bag for the shaft better than the best she owns (the mine's counter sells no
/// step back and nothing twice).
fn mine_gear_ahead(id: &str, inventory: &BTreeMap<String, u32>) -> bool {
    use dungeon::pozo::{Gear, BAGS, PICKS};
    let best = Gear::from_owned(|id| inventory.get(id).is_some_and(|n| *n > 0));
    if let Some(i) = PICKS.iter().position(|p| p.id == id) {
        return i > best.pick;
    }
    if let Some(i) = BAGS.iter().position(|b| b.id == id) {
        return i > best.bag;
    }
    false
}

/// The renderer and transactions share exactly the same filtered item ordering.
pub(super) fn visible_indices(
    tab: Tab,
    category: Category,
    inventory: &BTreeMap<String, u32>,
) -> Vec<usize> {
    let mine = counter() == Counter::Bocamina;
    catalog()
        .products
        .iter()
        .enumerate()
        .filter_map(|(i, p)| {
            let owned = inventory.get(&p.id).copied().unwrap_or(0) > 0;
            ((category == Category::All || p.category == category)
                && match (tab, mine) {
                    (Tab::Buy, false) => !p.sell_only && !p.hidden,
                    (Tab::Sell, false) => owned,
                    (Tab::Buy, true) => mine_gear_ahead(&p.id, inventory),
                    (Tab::Sell, true) => owned && dungeon::pozo::value(&p.id).is_some(),
                })
            .then_some(i)
        })
        .collect()
}

pub(super) fn page_selection(selection: usize, count: usize, delta: isize) -> usize {
    if count == 0 {
        return 0;
    }
    let last_page = (count - 1) / PAGE_SIZE;
    let next = (selection / PAGE_SIZE) as isize + delta;
    next.clamp(0, last_page as isize) as usize * PAGE_SIZE
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Tab {
    Buy,
    Sell,
}

impl Tab {
    pub(super) fn as_u8(self) -> u8 {
        match self {
            Self::Buy => 0,
            Self::Sell => 1,
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
pub(super) struct Product {
    pub id: String,
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub summary: String,
    #[serde(default)]
    pub effect: String,
    #[serde(default)]
    pub category: Category,
    #[serde(default)]
    pub queen_thought: Option<String>,
    pub icon: String,
    #[serde(default)]
    pub icon_svg: Option<String>,
    /// Optional raster illustration for product rows/details, relative to the shop asset directory.
    #[serde(default)]
    pub icon_image: Option<String>,
    /// Optional GLB preview, relative to the demo directory. Missing models use the icon.
    #[serde(default)]
    pub preview_model: Option<String>,
    pub buy_price: u32,
    pub sell_price: u32,
    /// Loot the shop buys but never stocks (materials from the calabozo).
    #[serde(default)]
    pub sell_only: bool,
    /// Kept in the catalog but not offered in this version (nothing uses it yet); she can still
    /// sell it if she has one.
    #[serde(default)]
    pub hidden: bool,
    #[serde(default)]
    pub stock: Option<u32>,
}

#[derive(Clone, Debug, Deserialize)]
pub(super) struct Catalog {
    #[allow(dead_code)]
    pub shop_id: String,
    // (the shop's own words, kept with its data: the interface names it by its place)
    #[allow(dead_code)]
    pub title: String,
    #[allow(dead_code)]
    pub currency: String,
    pub products: Vec<Product>,
}

pub(super) fn catalog() -> &'static Catalog {
    static DATA: OnceLock<Catalog> = OnceLock::new();
    DATA.get_or_init(|| {
        serde_json::from_str(include_str!("../data/shops/villa_tienda.json"))
            .expect("validated villa shop catalog")
    })
}

pub(super) fn default_stock() -> BTreeMap<String, u32> {
    catalog()
        .products
        .iter()
        .filter_map(|product| product.stock.map(|stock| (product.id.clone(), stock)))
        .collect()
}
