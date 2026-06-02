use uuid::Uuid;

pub fn correlation_id() -> String {
    Uuid::new_v4()
        .to_string()
}
