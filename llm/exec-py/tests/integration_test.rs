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
        let input = "cHJpbnQoJ2hlbGxvJyk="; // "print('hello')" in base64
        let result = base64::engine::general_purpose::STANDARD.decode(input).unwrap();
        assert_eq!(result, "print('hello')".as_bytes());
    }

    #[test]
    fn test_hex_decoding() {
        let input = "7072696e74282768656c6c6f2729"; // "print('hello')" in hex
        let result = hex::decode(input).unwrap();
        assert_eq!(result, "print('hello')".as_bytes());
    }
}


