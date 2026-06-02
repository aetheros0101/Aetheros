use chrono::{
    DateTime,
    Duration,
    Utc,
};

use uuid::Uuid;

pub struct ExecutionLease {
    pub execution_id:
        Uuid,

    pub lease_owner:
        Uuid,

    pub expires_at:
        DateTime<Utc>,
}

impl ExecutionLease {
    pub fn renew(
        &mut self,
        duration:
            Duration,
    ) {
        self.expires_at =
            Utc::now()
                + duration;
    }

    pub fn expired(
        &self,
    ) -> bool {
        Utc::now()
            > self.expires_at
    }
}
