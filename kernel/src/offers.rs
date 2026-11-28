//! Collected profile offers (TECHNICAL 9.10).

use crate::access::{self, OfferLink};
use crate::error::KernelError;
use crate::manage::{self, AccessQuestion, OfferedProfile, Pending};
use crate::sessions::{Host, Kernel};
use crate::state::State;
use std::collections::HashSet;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OfferInfo {
    pub name: String,
    pub description: String,
    pub digest: String,
    pub bytes: u32,
    pub adopted_as: Vec<String>,
}

fn collect(raw: Vec<OfferedProfile>) -> Vec<(OfferedProfile, String)> {
    let mut seen = HashSet::new();
    let mut kept = Vec::new();
    for offer in raw {
        if !manage::valid_name(&offer.name)
            || offer.description.len() > 256
            || offer.profile.len() > 65_536
            || !manage::valid_profile_text(&offer.profile)
        {
            continue;
        }
        let digest = access::digest(offer.profile.as_bytes());
        if seen.insert((offer.name.clone(), digest.clone())) {
            kept.push((offer, digest));
            if kept.len() == 64 {
                break;
            }
        }
    }
    kept
}

impl<S: State, H: Host> Kernel<S, H> {
    pub fn manage_offers(
        &mut self,
        caller: &str,
        after: u32,
        max: u32,
    ) -> Result<Vec<OfferInfo>, KernelError> {
        self.lease(caller)?;
        if max == 0 {
            return Err(manage::refusal("max", "must be positive"));
        }
        let offers = collect(self.host.offers_profiles());
        let mut index = access::index(&mut self.state);
        let mut changed = false;
        for row in &mut index {
            let Some(link) = &row.offer else {
                continue;
            };
            let same = offers
                .iter()
                .any(|(o, d)| o.name == link.name && d == &link.digest);
            let named = offers.iter().any(|(o, _)| o.name == link.name);
            let stale = row.stale || (named && !same);
            let gone = !named;
            if row.stale != stale || row.gone != gone {
                row.stale = stale;
                row.gone = gone;
                changed = true;
            }
        }
        if changed {
            manage::put(&mut self.state, "profiles", &index);
        }
        Ok(offers
            .into_iter()
            .skip(after as usize)
            .take(max.min(16) as usize)
            .map(|(o, digest)| {
                let adopted_as = index
                    .iter()
                    .filter(|row| {
                        row.offer
                            .as_ref()
                            .is_some_and(|link| link.name == o.name && link.digest == digest)
                    })
                    .map(|row| row.name.clone())
                    .collect();
                OfferInfo {
                    name: o.name,
                    description: o.description,
                    digest,
                    bytes: o.profile.len() as u32,
                    adopted_as,
                }
            })
            .collect())
    }

    pub fn manage_offer_text(
        &mut self,
        caller: &str,
        offer: &str,
        digest: &str,
    ) -> Result<String, KernelError> {
        self.lease(caller)?;
        collect(self.host.offers_profiles())
            .into_iter()
            .find(|(o, d)| o.name == offer && d == digest)
            .map(|(o, _)| o.profile)
            .ok_or_else(|| manage::refusal("offer", "no longer offered"))
    }

    pub fn manage_adopt(
        &mut self,
        caller: &str,
        offer: &str,
        digest: &str,
        as_name: &str,
    ) -> Result<String, KernelError> {
        self.lease(caller)?;
        if !manage::valid_name(as_name) {
            return Err(manage::refusal("as", "invalid profile name"));
        }
        let text = collect(self.host.offers_profiles())
            .into_iter()
            .find(|(o, d)| o.name == offer && d == digest)
            .map(|(o, _)| o.profile)
            .ok_or_else(|| manage::refusal("offer", "no longer offered"))?;
        let index = access::index_checked(&mut self.state)
            .ok_or_else(|| manage::refusal("profile", "unreadable profile index"))?;
        let old = index.iter().find(|row| row.name == as_name);
        if old.is_some_and(|row| row.offer.as_ref().is_none_or(|link| link.name != offer)) {
            return Err(manage::refusal("as", "profile belongs to another offer"));
        }
        if old.is_none() && index.len() >= 256 {
            return Err(manage::refusal("as", "256 profiles"));
        }
        if old.is_some_and(|row| row.rev == u32::MAX) {
            return Err(manage::refusal("profile", "version limit reached"));
        }
        let mut summary = manage::reach(&text, None)?;
        summary.push_str("\nAdopted from an offer");
        if old.is_some() {
            let next = manage::field_digests(&text)
                .ok_or_else(|| manage::refusal("profile", "invalid profile"))?;
            let previous = self
                .state
                .get(&format!("profile/{as_name}"))
                .and_then(|bytes| String::from_utf8(bytes).ok())
                .and_then(|s| manage::field_digests(&s));
            let fields = manage::changed_fields(previous.as_ref(), &next);
            summary.push_str(&format!("\nChanged: {}", fields.join(", ")));
        } else {
            summary.push_str("\nNew profile");
        }
        if summary.len() > 4096 {
            return Err(manage::refusal("profile", "too wide to show"));
        }
        self.question(
            AccessQuestion::Store {
                profile: as_name.into(),
                text: summary,
            },
            Pending::Store {
                profile: as_name.into(),
                digest: digest.into(),
                base: old.map(|row| row.digest.clone()),
                text,
                offer: Some(OfferLink {
                    name: offer.into(),
                    digest: digest.into(),
                }),
            },
        )?;
        Ok(digest.into())
    }
}

#[cfg(test)]
mod tests {
    use crate::manage::OfferedProfile;
    use crate::{FakeHost, Kernel, MemState};

    #[test]
    fn ac1_collect_bounds_deduplicates_and_pages() {
        let mut k = Kernel::new(
            MemState::new(),
            FakeHost {
                connection_id: Some("c".into()),
                ..Default::default()
            },
        );
        k.manage_open("page").unwrap();
        k.manage_on_item("t1", &serde_json::json!({"type":"granted"}));
        k.host.offered_profiles = (0..65)
            .map(|i| OfferedProfile {
                name: format!("p{i}"),
                description: "d".into(),
                profile: r#"{"backend":"claude","tools":[]}"#.into(),
            })
            .collect();
        k.host
            .offered_profiles
            .push(k.host.offered_profiles[0].clone());
        assert_eq!(k.manage_offers("page", 0, 16).unwrap().len(), 16);
        let first = k.manage_offers("page", 0, 1).unwrap().remove(0);
        assert_eq!(first.name, "p0");
        assert_eq!(
            first.bytes as usize,
            k.manage_offer_text("page", "p0", &first.digest)
                .unwrap()
                .len()
        );
        assert_eq!(k.manage_offers("page", 48, 16).unwrap().len(), 16);
        assert!(k.manage_offers("page", 64, 16).unwrap().is_empty());
    }

    #[test]
    fn ac2_adopt_answer_links_and_change_stales_until_readoption() {
        let mut k = Kernel::new(
            MemState::new(),
            FakeHost {
                connection_id: Some("c".into()),
                ..Default::default()
            },
        );
        k.manage_open("page").unwrap();
        k.manage_on_item("t1", &serde_json::json!({"type":"granted"}));
        let text = r#"{"backend":"claude","tools":[]}"#;
        k.host.offered_profiles.push(OfferedProfile {
            name: "p".into(),
            description: "d".into(),
            profile: text.into(),
        });
        let digest = k.manage_offers("page", 0, 1).unwrap()[0].digest.clone();
        k.manage_adopt("page", "p", &digest, "mine").unwrap();
        assert!(k.manage_profiles("page", None, 10).unwrap().is_empty());
        k.manage_on_item("t2", &serde_json::json!({"type":"granted"}));
        assert_eq!(k.manage_profile("page", "mine").unwrap(), text);
        k.host.offered_profiles[0].profile =
            r#"{"backend":"claude","tools":[],"max-turns":2}"#.into();
        k.manage_offers("page", 0, 1).unwrap();
        assert!(k.manage_profiles("page", None, 10).unwrap()[0].stale);
        assert_eq!(
            k.manage_allow("page", "page", "mine").unwrap_err().code(),
            "refused"
        );
        k.host.offered_profiles.clear();
        k.manage_offers("page", 0, 1).unwrap();
        assert!(k.manage_profiles("page", None, 10).unwrap()[0].stale);
        k.host.offered_profiles.push(OfferedProfile {
            name: "p".into(),
            description: "d".into(),
            profile: text.into(),
        });
        k.manage_adopt("page", "p", &digest, "mine").unwrap();
        k.manage_on_item("t3", &serde_json::json!({"type":"granted"}));
        let row = &k.manage_profiles("page", None, 10).unwrap()[0];
        assert!(!row.stale);
        assert_eq!(row.rev, 2);
    }
}
