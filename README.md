# router_prefilter

Fast prefix-based prefiltering for router pattern matching.

This crate provides efficient prefiltering of route matchers by extracting and indexing literal prefixes from patterns.

## Usage

```rust
use router_prefilter::RouterPrefilter;
use router_prefilter::matchers::{Matcher, MatcherVisitor};

struct RoutePattern {
    regex: String,
}

impl Matcher for RoutePattern {
    fn visit(&self, visitor: &mut MatcherVisitor) {
        visitor.visit_match_regex(&self.regex);
    }
}

let routes = vec![
    RoutePattern { regex: r"^/items/\d+$".to_string() },
    RoutePattern { regex: r"^/users?/.*$".to_string() },
];

let mut prefilter = RouterPrefilter::new();
for (i, route) in routes.into_iter().enumerate() {
    prefilter.insert(i, route);
}

let matches: Vec<_> = prefilter.possible_matches("/items/100").collect();
assert_eq!(matches, vec![&0]);
```

## How It Works

The prefilter analyzes route matchers to extract literal prefixes and builds an efficient data structure for fast lookup.
The route matcher must implement the `Matcher` trait, and must call the specified functions on the passed visitor.
This can include nesting, multiple patterns `AND`ed and `OR`ed together, etc.

If we are able to find a small set of prefixes where one _must_ match at the beginning of the actual string in order for the overall pattern to match, those found prefixes will be used as a prefilter.
e.g. `path startsWith "/abc" OR path matchesRegex "^/ef[gh]"`: the path must start with either `/abc`, `/efg` or `/efh`.
Alternations which cannot be proved to start with anything will cause prefiltering to be impossible for that route.
e.g. `path startsWith "/abc" OR path endsWith "zzz"`: prefiltering is impossible because the path could start with _anything_.
