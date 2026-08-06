#![no_main]

mod pest_parser;

use libfuzzer_sys::fuzz_target;

fuzz_target!(|input: &str| {
    // Parse with both parsers
    let result1 = pest_parser::parse(input);
    let result2 = atc_router::parser::parse(input);

    // Compare results
    match (&result1, &result2) {
        (Ok(ast1), Ok(ast2)) => {
            // Both succeeded - ASTs must be identical
            assert_eq!(
                ast1, ast2,
                "Parsers produced different ASTs for input: {:?}",
                input
            );
        }
        (Err(_), Err(_)) => {
            // Both failed - this is acceptable
            // We don't compare error messages as they may differ between implementations
        }
        (Ok(ast), Err(err)) => {
            panic!(
                "Parser 1 (pest) succeeded but Parser 2 (winnow) failed!\nInput: {:?}\nAST: {:?}\nError: {:?}",
                input, ast, err
            );
        }
        (Err(err), Ok(ast)) => {
            panic!(
                "Parser 1 (pest) failed but Parser 2 (winnow) succeeded!\nInput: {:?}\nError: {:?}\nAST: {:?}",
                input, err, ast
            );
        }
    }
    if let Err(e) = result2 {
        let inner = e.inner();
        if input != "" && inner.context().count() == 0 {
            panic!("no context")
        }
    }
});
