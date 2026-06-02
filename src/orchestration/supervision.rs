use std::collections::HashMap;

use uuid::Uuid;

pub struct SupervisionTree {
    parents:
        HashMap<Uuid, Uuid>,
}

impl SupervisionTree {
    pub fn new() -> Self {
        Self {
            parents:
                HashMap::new(),
        }
    }

    pub fn attach(
        &mut self,
        child: Uuid,

        parent: Uuid,
    ) {
        self.parents.insert(
            child,
            parent,
        );
    }

    pub fn parent(
        &self,
        child: &Uuid,
    ) -> Option<&Uuid> {
        self.parents.get(child)
    }
}
