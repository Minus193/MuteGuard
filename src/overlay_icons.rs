use std::{collections::HashMap, sync::LazyLock};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OverlayIconPair {
    pub id: &'static str,
    pub label: &'static str,
    pub unmuted_svg: &'static str,
    pub muted_svg: &'static str,
}

macro_rules! pair {
    ($id:literal, $label:literal, $unmuted:literal, $muted:literal) => {
        OverlayIconPair {
            id: $id,
            label: $label,
            unmuted_svg: include_str!(concat!("../assets/icons/overlay/", $unmuted)),
            muted_svg: include_str!(concat!("../assets/icons/overlay/", $muted)),
        }
    };
}

const ICON_PAIRS: [OverlayIconPair; 17] = [
    pair!("fluent", "Fluent", "fluent-mic.svg", "fluent-mic-off.svg"),
    pair!(
        "solar",
        "Solar",
        "solar-microphone-3-linear.svg",
        "solar-microphone-3-broken.svg"
    ),
    pair!(
        "phosphor",
        "Phosphor",
        "ph-microphone.svg",
        "ph-microphone-slash.svg"
    ),
    pair!(
        "hugeicons",
        "Hugeicons",
        "hugeicons-mic-01.svg",
        "hugeicons-mic-off-01.svg"
    ),
    pair!("lucide", "Lucide", "lucide-mic.svg", "lucide-mic-off.svg"),
    pair!(
        "tabler",
        "Tabler",
        "tabler-microphone.svg",
        "tabler-microphone-off.svg"
    ),
    pair!(
        "material",
        "Material",
        "material-mic-outline.svg",
        "material-mic-off-outline.svg"
    ),
    pair!("mdi", "MDI", "mdi-microphone.svg", "mdi-microphone-off.svg"),
    pair!(
        "remix",
        "Remix",
        "remix-mic-line.svg",
        "remix-mic-off-line.svg"
    ),
    pair!(
        "iconamoon",
        "IconMoon",
        "iconamoon-microphone.svg",
        "iconamoon-microphone-off.svg"
    ),
    pair!(
        "gravity",
        "Gravity",
        "gravity-microphone.svg",
        "gravity-microphone-slash.svg"
    ),
    pair!(
        "eva",
        "Eva",
        "eva-mic-outline.svg",
        "eva-mic-off-outline.svg"
    ),
    pair!(
        "uicons",
        "UIcons",
        "uil-microphone.svg",
        "uil-microphone-slash.svg"
    ),
    pair!(
        "basil",
        "Basil",
        "basil-microphone-outline.svg",
        "basil-microphone-off-outline.svg"
    ),
    pair!(
        "pepicons",
        "Pepicons",
        "pepicons-microphone.svg",
        "pepicons-microphone-off.svg"
    ),
    pair!(
        "mingcute",
        "MingCute",
        "mingcute-mic.svg",
        "mingcute-mic-off.svg"
    ),
    pair!(
        "mingcute-fill",
        "Ming Fill",
        "mingcute-mic-fill.svg",
        "mingcute-mic-off-fill.svg"
    ),
];

static FEATURED_ICON_PAIRS: LazyLock<Vec<OverlayIconPair>> = LazyLock::new(|| {
    let featured_ids = ["mdi", "fluent", "lucide", "phosphor", "solar"];
    featured_ids
        .iter()
        .map(|id| *overlay_icon_pair(id))
        .collect()
});

static EXTRA_ICON_PAIRS: LazyLock<Vec<OverlayIconPair>> = LazyLock::new(|| {
    let featured_ids = ["mdi", "fluent", "lucide", "phosphor", "solar"];
    ICON_PAIRS
        .iter()
        .copied()
        .filter(|pair| !featured_ids.contains(&pair.id))
        .collect()
});

static ICON_CSS_URLS: LazyLock<HashMap<(&'static str, bool), String>> = LazyLock::new(|| {
    let mut urls = HashMap::with_capacity(ICON_PAIRS.len() * 2);
    for pair in ICON_PAIRS {
        urls.insert((pair.id, false), svg_css_url(pair.unmuted_svg));
        urls.insert((pair.id, true), svg_css_url(pair.muted_svg));
    }
    urls
});

pub fn default_overlay_icon_pair() -> String {
    "mdi".to_string()
}

pub fn featured_overlay_icon_pairs() -> &'static [OverlayIconPair] {
    &FEATURED_ICON_PAIRS
}

pub fn extra_overlay_icon_pairs() -> &'static [OverlayIconPair] {
    &EXTRA_ICON_PAIRS
}

pub fn extra_overlay_icon_pair(id: &str) -> Option<OverlayIconPair> {
    EXTRA_ICON_PAIRS.iter().find(|pair| pair.id == id).copied()
}

pub fn overlay_icon_pair(id: &str) -> &'static OverlayIconPair {
    ICON_PAIRS
        .iter()
        .find(|pair| pair.id == id)
        .unwrap_or(&ICON_PAIRS[0])
}

pub fn overlay_icon_svg(id: &str, muted: bool) -> &'static str {
    let pair = overlay_icon_pair(id);
    if muted {
        pair.muted_svg
    } else {
        pair.unmuted_svg
    }
}

pub fn overlay_icon_css_url(id: &str, muted: bool) -> &'static str {
    let pair = overlay_icon_pair(id);
    ICON_CSS_URLS
        .get(&(pair.id, muted))
        .map(String::as_str)
        .expect("every overlay icon has a cached CSS URL")
}

fn svg_css_url(svg: &str) -> String {
    format!("data:image/svg+xml;utf8,{}", encode_svg(svg))
}

fn encode_svg(svg: &str) -> String {
    let mut encoded = String::with_capacity(svg.len() * 2);
    for byte in svg.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                encoded.push(byte as char)
            }
            _ => {
                encoded.push('%');
                encoded.push_str(&format!("{byte:02X}"));
            }
        }
    }
    encoded
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn icon_groups_are_complete_disjoint_and_ordered() {
        let recommended = featured_overlay_icon_pairs()
            .iter()
            .map(|pair| pair.id)
            .collect::<Vec<_>>();
        let more = extra_overlay_icon_pairs()
            .iter()
            .map(|pair| pair.id)
            .collect::<Vec<_>>();

        assert_eq!(
            recommended,
            ["mdi", "fluent", "lucide", "phosphor", "solar"]
        );
        assert_eq!(recommended.len() + more.len(), ICON_PAIRS.len());
        assert!(recommended.iter().all(|id| !more.contains(id)));
        assert_eq!(default_overlay_icon_pair(), "mdi");
    }
}
