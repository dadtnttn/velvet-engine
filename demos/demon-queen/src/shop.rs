//! Data-driven shop catalog and shared shop state types.
//!
//! The villa shop UI reads its products from `data/shops/villa_tienda.json`.
//! Adding or rebalancing an item therefore only changes data; the renderer and
//! transaction code do not need a new match arm.

use super::*;

pub(super) const PAGE_SIZE: usize = 8;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize, Hash)]
#[serde(rename_all = "snake_case")]
pub(super) enum Category {
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
    pub fn icon(self) -> &'static str {
        match self {
            Self::All => "category_all",
            Self::Consumables => "tab_sell",
            Self::Equipment => "category_equipment",
            Self::Materials => "category_materials",
            Self::Special => "category_special",
        }
    }
}

/// The renderer and transactions share exactly the same filtered item ordering.
pub(super) fn visible_indices(
    tab: Tab,
    category: Category,
    inventory: &BTreeMap<String, u32>,
) -> Vec<usize> {
    catalog()
        .products
        .iter()
        .enumerate()
        .filter_map(|(i, p)| {
            ((category == Category::All || p.category == category)
                && match tab {
                    Tab::Buy => !p.sell_only && !p.hidden,
                    Tab::Sell => inventory.get(&p.id).copied().unwrap_or(0) > 0,
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
pub(super) enum Tab {
    Buy,
    Sell,
}

impl Tab {
    pub(super) fn toggled(self) -> Self {
        match self {
            Self::Buy => Self::Sell,
            Self::Sell => Self::Buy,
        }
    }

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
    pub title: String,
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
