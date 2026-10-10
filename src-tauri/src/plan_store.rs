//! Server-side store for previewed plans.
//!
//! The webview only ever holds a `plan_id`. The paths that are executed come
//! from the plan kept here, so a compromised or buggy renderer cannot make the
//! backend move arbitrary files by sending its own plan.

use crate::planner::OrganizationPlan;
use std::{
    collections::{HashMap, VecDeque},
    sync::Mutex,
};

/// A handful of previews is plenty: the UI works on one plan at a time.
const MAX_PLANS: usize = 8;

#[derive(Default)]
struct Inner {
    plans: HashMap<String, OrganizationPlan>,
    order: VecDeque<String>,
}

#[derive(Default)]
pub struct PlanStore(Mutex<Inner>);

const UNKNOWN_PLAN: &str = "정리 계획을 찾을 수 없습니다. 이미 실행했거나 만료되었습니다. 폴더를 다시 분석하세요. / The plan was not found: it was already run or has expired. Analyse the folder again.";

impl PlanStore {
    fn lock(&self) -> std::sync::MutexGuard<'_, Inner> {
        // A panic while holding the lock cannot leave the maps half-updated.
        self.0
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// Store `plan` under a fresh id (written into `plan.plan_id`).
    pub fn insert(&self, plan: &mut OrganizationPlan) {
        plan.plan_id = uuid::Uuid::new_v4().to_string();
        self.put(plan.clone());
    }

    fn put(&self, plan: OrganizationPlan) {
        let mut inner = self.lock();
        let id = plan.plan_id.clone();
        if inner.plans.insert(id.clone(), plan).is_none() {
            inner.order.push_back(id);
        }
        while inner.order.len() > MAX_PLANS {
            if let Some(oldest) = inner.order.pop_front() {
                inner.plans.remove(&oldest);
            }
        }
    }

    pub fn get(&self, plan_id: &str) -> Result<OrganizationPlan, String> {
        self.lock()
            .plans
            .get(plan_id)
            .cloned()
            .ok_or_else(|| UNKNOWN_PLAN.to_string())
    }

    /// Replace a stored plan after an edit (for example approved AI suggestions).
    pub fn update(&self, plan: OrganizationPlan) -> Result<(), String> {
        let mut inner = self.lock();
        match inner.plans.get_mut(&plan.plan_id) {
            Some(slot) => {
                *slot = plan;
                Ok(())
            }
            None => Err(UNKNOWN_PLAN.to_string()),
        }
    }

    /// Remove a plan for execution. A plan can be claimed once, so a double
    /// click or a replayed request cannot run it twice.
    pub fn claim(&self, plan_id: &str) -> Result<OrganizationPlan, String> {
        let mut inner = self.lock();
        let plan = inner
            .plans
            .remove(plan_id)
            .ok_or_else(|| UNKNOWN_PLAN.to_string())?;
        inner.order.retain(|id| id != plan_id);
        Ok(plan)
    }

    /// Put a claimed plan back when execution could not start.
    pub fn restore(&self, plan: OrganizationPlan) {
        self.put(plan);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plan() -> OrganizationPlan {
        OrganizationPlan {
            root_path: "C:\\x".into(),
            actions: Vec::new(),
            exclusions: Vec::new(),
            errors: Vec::new(),
            warnings: Vec::new(),
            created_at: String::new(),
            basis: String::new(),
            language: String::new(),
            mode: String::new(),
            root_paths: Vec::new(),
            run_id: String::new(),
            plan_id: String::new(),
        }
    }

    #[test]
    fn insert_assigns_an_id_and_get_returns_a_copy() {
        let store = PlanStore::default();
        let mut stored = plan();
        store.insert(&mut stored);
        assert_eq!(stored.plan_id.len(), 36);
        assert_eq!(store.get(&stored.plan_id).unwrap().plan_id, stored.plan_id);
        assert!(store.get("not-a-plan").is_err());
    }

    #[test]
    fn a_plan_can_be_claimed_only_once() {
        let store = PlanStore::default();
        let mut stored = plan();
        store.insert(&mut stored);
        let claimed = store.claim(&stored.plan_id).expect("first claim");
        assert!(
            store.claim(&stored.plan_id).is_err(),
            "second claim must fail"
        );
        store.restore(claimed);
        assert!(
            store.claim(&stored.plan_id).is_ok(),
            "restored plans can be claimed again"
        );
    }

    #[test]
    fn only_the_newest_plans_are_kept() {
        let store = PlanStore::default();
        let mut first = plan();
        store.insert(&mut first);
        for _ in 0..MAX_PLANS {
            store.insert(&mut plan());
        }
        assert!(
            store.get(&first.plan_id).is_err(),
            "the oldest plan is evicted"
        );
    }

    #[test]
    fn update_requires_an_existing_plan() {
        let store = PlanStore::default();
        let mut stored = plan();
        store.insert(&mut stored);
        stored.warnings.push("edited".into());
        store.update(stored.clone()).expect("update");
        assert_eq!(store.get(&stored.plan_id).unwrap().warnings, vec!["edited"]);
        let mut stranger = plan();
        stranger.plan_id = "missing".into();
        assert!(store.update(stranger).is_err());
    }
}
