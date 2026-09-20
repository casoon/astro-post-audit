//! `a11y-dom` über `scraper::Html`.
//!
//! Der Adapter macht den geparsten Baum dieses Werkzeugs für den gemeinsamen
//! Regelbestand lesbar. Er enthält selbst keine Regel — die kommen aus
//! `a11y-rules` und heißen dort so wie in `auditmysite` und in LiveAudit.
//!
//! # Welche Tiers bedient werden
//!
//! **Struktur** und **Semantik**. Die Semantik kommt über `accname`, das
//! generisch über `a11y_dom::Node` arbeitet und damit auch hier funktioniert:
//! Rolle und Accessible Name werden nach HTML-AAM und accname 1.2 berechnet,
//! nicht aus Teilbaumtext geraten.
//!
//! **Darstellung nicht.** Statische HTML-Analyse kennt keine berechneten Stile
//! und keine Geometrie. Die Kontrastregeln melden deshalb `UNTESTED` — nicht
//! `PASS` und nicht Schweigen. Genau das ist der Unterschied zu einem
//! Werkzeug, das eine nicht geprüfte Regel als bestanden zählt.

use std::collections::HashMap;

use a11y_dom::{Document, NameSource, NodeId, NodeKind, Semantics};
use accname::IdIndex;
use ego_tree::NodeRef;
use scraper::{Html, Node as ScraperNode};

/// `ego_tree::NodeId` gibt seinen Index nicht her — er ist bewusst opak.
///
/// Der Index wird deshalb einmal je Dokument vergeben, in Dokumentreihenfolge.
/// `a11y-report` führt ihn als `location.node` mit; für dieses Werkzeug ist er
/// nur ein Rückbezug, weil Befunde hier über Datei und Selektor verortet
/// werden.
struct NodeIndex {
    ids: HashMap<ego_tree::NodeId, u32>,
}

impl NodeIndex {
    fn build(root: NodeRef<'_, ScraperNode>) -> Self {
        let mut ids = HashMap::new();
        let mut stack = vec![root];
        let mut next = 0u32;
        while let Some(n) = stack.pop() {
            if !ist_relevant(n) {
                continue;
            }
            ids.insert(n.id(), next);
            next += 1;
            // Umgekehrt auf den Stapel, damit die Reihenfolge stimmt.
            for kind in n.children().collect::<Vec<_>>().into_iter().rev() {
                stack.push(kind);
            }
        }
        NodeIndex { ids }
    }

    fn of(&self, id: ego_tree::NodeId) -> NodeId {
        NodeId(self.ids.get(&id).copied().unwrap_or(u32::MAX))
    }
}

/// Kommentare, Doctype und Processing Instructions kommen im Modell von
/// `a11y-dom` nicht vor — für Accessibility-Regeln sind sie ohne Bedeutung.
fn ist_relevant(n: NodeRef<'_, ScraperNode>) -> bool {
    matches!(n.value(), ScraperNode::Element(_) | ScraperNode::Text(_))
}

/// Ein Knoten im `scraper`-Baum, angereichert um die Indextabelle.
///
/// `Copy`, weil `a11y_dom::Node` es verlangt: Die Traversierungs-Iteratoren
/// erzeugen viele kurzlebige Handles. Die Gleichheit vergleicht **nur** den
/// Baumknoten — würde sie die Tabelle mitvergleichen, vergliche sie bei jedem
/// `==` zwei HashMaps.
pub struct Knoten<'a> {
    inner: NodeRef<'a, ScraperNode>,
    index: &'a NodeIndex,
}

impl Clone for Knoten<'_> {
    fn clone(&self) -> Self {
        *self
    }
}

impl Copy for Knoten<'_> {}

impl PartialEq for Knoten<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.inner == other.inner
    }
}

impl Eq for Knoten<'_> {}

impl<'a> a11y_dom::Node<'a> for Knoten<'a> {
    fn id(self) -> NodeId {
        self.index.of(self.inner.id())
    }

    fn kind(self) -> NodeKind {
        match self.inner.value() {
            ScraperNode::Text(_) => NodeKind::Text,
            _ => NodeKind::Element,
        }
    }

    fn parent(self) -> Option<Self> {
        let p = self.inner.parent()?;
        ist_relevant(p).then_some(Knoten {
            inner: p,
            index: self.index,
        })
    }

    fn children(self) -> impl Iterator<Item = Self> + 'a {
        let index = self.index;
        self.inner
            .children()
            .filter(|c| ist_relevant(*c))
            .map(move |inner| Knoten { inner, index })
    }

    fn local_name(self) -> &'a str {
        match self.inner.value() {
            ScraperNode::Element(el) => el.name(),
            _ => "",
        }
    }

    fn attributes(self) -> impl Iterator<Item = (&'a str, &'a str)> + 'a {
        let attrs = match self.inner.value() {
            ScraperNode::Element(el) => Some(el.attrs()),
            _ => None,
        };
        attrs.into_iter().flatten()
    }

    fn text(self) -> &'a str {
        match self.inner.value() {
            ScraperNode::Text(t) => &t.text,
            _ => "",
        }
    }

    /// `scraper` hält die Attribute in einer Map — die lineare Suche des
    /// Vorgabepfads wäre hier unnötig.
    fn attr(self, name: &str) -> Option<&'a str> {
        match self.inner.value() {
            ScraperNode::Element(el) => el.attr(name),
            _ => None,
        }
    }
}

/// Ein geparstes Dokument samt Indextabelle und Namensauflösung.
///
/// Der `IdIndex` entsteht **einmal je Dokument**. Ihn je Knoten neu aufzubauen
/// machte die Namensauflösung quadratisch — dieselbe Auflage wie in LiveAudit.
pub struct Seite<'a> {
    html: &'a Html,
    index: NodeIndex,
}

impl<'a> Seite<'a> {
    pub fn new(html: &'a Html) -> Self {
        Seite {
            html,
            index: NodeIndex::build(*html.root_element()),
        }
    }
}

impl Document for Seite<'_> {
    type N<'n>
        = Knoten<'n>
    where
        Self: 'n;

    fn root(&self) -> Self::N<'_> {
        Knoten {
            inner: *self.html.root_element(),
            index: &self.index,
        }
    }

    fn node_count(&self) -> Option<usize> {
        Some(self.index.ids.len())
    }
}

/// Die Seite plus der einmal je Dokument gebaute ID-Index für `accname`.
///
/// Hält die `Seite` als **Referenz**, nicht als Besitz. Andernfalls wäre die
/// Struktur selbstbezüglich: Der Index verweist auf Knoten, die ihrerseits auf
/// die Indextabelle in der `Seite` zeigen. Mit der Referenz liegt beides
/// getrennt beim Aufrufer, und es braucht kein `unsafe` — dieselbe Aufteilung
/// wie in LiveAudit.
pub struct SeiteMitNamen<'a> {
    seite: &'a Seite<'a>,
    ids: IdIndex<'a, Knoten<'a>>,
}

impl<'a> SeiteMitNamen<'a> {
    pub fn new(seite: &'a Seite<'a>) -> Self {
        SeiteMitNamen {
            ids: IdIndex::build(seite.root()),
            seite,
        }
    }
}

impl Document for SeiteMitNamen<'_> {
    type N<'n>
        = Knoten<'n>
    where
        Self: 'n;

    fn root(&self) -> Self::N<'_> {
        self.seite.root()
    }

    fn node_count(&self) -> Option<usize> {
        self.seite.node_count()
    }
}

impl Semantics for SeiteMitNamen<'_> {
    fn role<'n>(&'n self, node: Self::N<'n>) -> Option<String> {
        accname::role(node).map(str::to_string)
    }

    fn accessible_name<'n>(&'n self, node: Self::N<'n>) -> Option<String> {
        accname::name(node, &self.ids)
    }

    /// Ohne nativen Accessibility-Tree ist nicht bekannt, *woher* ein Name
    /// stammt. `None` ist die ehrliche Antwort, nicht eine Lücke.
    fn name_source<'n>(&'n self, _node: Self::N<'n>) -> Option<NameSource> {
        None
    }

    fn is_ignored<'n>(&'n self, node: Self::N<'n>) -> bool {
        use a11y_dom::Node;
        node.attr("aria-hidden") == Some("true")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use a11y_dom::{elements, Node};

    fn parse(s: &str) -> Html {
        Html::parse_document(s)
    }

    #[test]
    fn wurzel_ist_das_html_element() {
        let html = parse("<!doctype html><html lang=\"de\"><body></body></html>");
        let seite = Seite::new(&html);
        assert_eq!(seite.root().local_name(), "html");
        assert_eq!(seite.root().attr("lang"), Some("de"));
    }

    #[test]
    fn kommentare_und_doctype_kommen_nicht_vor() {
        let html = parse("<!doctype html><html><body><!-- weg --><p>Text</p></body></html>");
        let seite = Seite::new(&html);
        let namen: Vec<&str> = elements(&seite).map(|n| n.local_name()).collect();
        assert!(namen.contains(&"p"));
        assert!(!namen.contains(&""), "ein Nicht-Element ist durchgerutscht");
    }

    #[test]
    fn knotenkennungen_sind_eindeutig_und_in_dokumentreihenfolge() {
        let html = parse("<html><body><h1>A</h1><p>B</p></body></html>");
        let seite = Seite::new(&html);
        let ids: Vec<u32> = elements(&seite).map(|n| n.id().0).collect();
        let mut sortiert = ids.clone();
        sortiert.sort_unstable();
        sortiert.dedup();
        assert_eq!(ids.len(), sortiert.len(), "doppelte Kennung");
        assert_eq!(ids, sortiert, "nicht in Dokumentreihenfolge");
    }

    #[test]
    fn text_steht_an_den_textknoten_nicht_an_den_elementen() {
        let html = parse("<html><body><p>Hallo</p></body></html>");
        let seite = Seite::new(&html);
        let p = elements(&seite).find(|n| n.local_name() == "p").unwrap();
        assert_eq!(p.text(), "");
        assert_eq!(a11y_dom::subtree_text(p), "Hallo");
    }

    #[test]
    fn attribute_kommen_vollstaendig_an() {
        let html = parse("<html><body><img src=\"a.png\" alt=\"Ein Bild\"></body></html>");
        let seite = Seite::new(&html);
        let img = elements(&seite).find(|n| n.local_name() == "img").unwrap();
        assert_eq!(img.attr("src"), Some("a.png"));
        assert_eq!(img.attr("alt"), Some("Ein Bild"));
        assert_eq!(img.attr("title"), None);
        assert_eq!(img.attributes().count(), 2);
    }

    #[test]
    fn eltern_und_kinder_haengen_zusammen() {
        let html = parse("<html><body><div><span>x</span></div></body></html>");
        let seite = Seite::new(&html);
        let span = elements(&seite).find(|n| n.local_name() == "span").unwrap();
        assert_eq!(span.parent().unwrap().local_name(), "div");
    }

    /// Der eigentliche Gewinn des Adapters: Der Accessible Name entsteht nach
    /// accname, nicht aus Teilbaumtext. Über `aria-labelledby` ist er sonst
    /// nicht zu bekommen.
    #[test]
    fn accessible_name_kommt_ueber_aria_labelledby() {
        let html = parse(
            r#"<html><body><span id="l">Weiterlesen</span><a href="/a" aria-labelledby="l"></a></body></html>"#,
        );
        let seite = Seite::new(&html);
        let doc = SeiteMitNamen::new(&seite);
        let a = elements(&doc).find(|n| n.local_name() == "a").unwrap();
        assert_eq!(doc.accessible_name(a).as_deref(), Some("Weiterlesen"));
    }

    #[test]
    fn rolle_wird_abgeleitet() {
        let html = parse(r#"<html><body><a href="/a">x</a></body></html>"#);
        let seite = Seite::new(&html);
        let doc = SeiteMitNamen::new(&seite);
        let a = elements(&doc).find(|n| n.local_name() == "a").unwrap();
        assert_eq!(doc.role(a).as_deref(), Some("link"));
    }
}
