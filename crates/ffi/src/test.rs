#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_session_create_and_add_message() {
        let session = Session::new("test-session".to_string());
        session.add_message("user".to_string(), "Hello".to_string()).await.unwrap();
        let messages = session.get_messages().await.unwrap();
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].role, "user");
        assert_eq!(messages[0].content, "Hello");
    }
}