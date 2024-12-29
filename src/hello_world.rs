pub fn get_hw_string() -> &'static str {
    "Hello world!"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_hw_string() {
        assert_eq!(get_hw_string(), "Hello world!")
    }
}
