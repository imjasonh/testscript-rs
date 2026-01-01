#![no_main]

use libfuzzer_sys::fuzz_target;
use testscript_rs::TestEnvironment;

fuzz_target!(|data: &[u8]| {
    // Convert bytes to string, handling invalid UTF-8 gracefully
    let input = String::from_utf8_lossy(data);

    // Create a test environment to test environment variable substitution
    if let Ok(mut env) = TestEnvironment::new() {
        // Add some test environment variables with potentially problematic values
        let test_vars = vec![
            ("WORK", "/tmp/work"),
            ("HOME", "/home/user"),
            ("PATH", "/usr/bin:/bin"),
            ("SPECIAL", "${}()[].*^$\\"),
            ("EMPTY", ""),
            ("DOLLAR", "$"),
            ("NESTED", "${OTHER}"),
            ("OTHER", "value"),
        ];

        for (key, value) in test_vars {
            env.env_vars.insert(key.to_string(), value.to_string());
        }

        // Test environment variable substitution
        // This should never panic, regardless of input
        let result = env.substitute_env_vars(&input);

        // Basic sanity check - result should be a valid string
        let _len = result.len();

        // Test with some edge cases
        let edge_cases = vec![
            format!("${{{}}}", input),     // ${input}
            format!("${}", input),         // $input
            format!("{}${{WORK}}", input), // input${WORK}
            format!("${{WORK}}{}", input), // ${WORK}input
            format!("$${}$$", input),      // $$input$$
        ];

        for edge_case in edge_cases {
            let _result = env.substitute_env_vars(&edge_case);
            // Should not panic
        }

        // Test that substitution is idempotent for non-recursive cases
        let once = env.substitute_env_vars(&input);
        let twice = env.substitute_env_vars(&once);

        // For most inputs, applying substitution twice should yield the same result
        // unless the first substitution introduced new variables to substitute
        if !once.contains('$') {
            assert_eq!(
                once, twice,
                "Substitution should be idempotent when no $ remains"
            );
        }
    }
});
