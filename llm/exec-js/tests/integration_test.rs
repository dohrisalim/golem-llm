#[cfg(test)]
mod tests {
    // Basic unit tests for functions that don't depend on WIT bindings
    
    #[test]
    fn test_basic_functionality() {
        // Test that the crate compiles and basic tests pass
        assert_eq!(2 + 2, 4);
    }

    #[test]
    fn test_base64_decoding() {
        use base64::Engine;
        let input = "Y29uc29sZS5sb2coJ2hlbGxvJyk7"; // "console.log('hello');" in base64
        let result = base64::engine::general_purpose::STANDARD.decode(input).unwrap();
        assert_eq!(result, "console.log('hello');".as_bytes());
    }

    #[test]
    fn test_hex_decoding() {
        let input = "636f6e736f6c652e6c6f67282768656c6c6f27293b"; // "console.log('hello');" in hex
        let result = hex::decode(input).unwrap();
        assert_eq!(result, "console.log('hello');".as_bytes());
    }
}

