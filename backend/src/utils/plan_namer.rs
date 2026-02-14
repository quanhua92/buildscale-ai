//! Generates random 3-word hyphenated names for plan files.

use rand::Rng;

const ADJECTIVES: &[&str] = &[
    "able", "brave", "calm", "clean", "clear", "cute", "daring", "eager",
    "fancy", "gentle", "happy", "jolly", "kind", "lively", "merry", "nice",
    "polite", "quick", "rapid", "sharp", "tidy", "vivid", "warm", "young",
    "bold", "bright", "crisp", "fresh", "grand", "keen", "loud", "pure",
    "rich", "safe", "soft", "swift", "vast", "wild", "witty", "zesty",
    "azure", "crimson", "golden", "silver", "emerald", "ruby", "amber",
    "jubilant", "gleeful", "serene", "vibrant", "cosmic", "ethereal",
    "frosty", "misty", "sunny", "stormy", "radiant", "luminous", "stellar",
];

const NOUNS: &[&str] = &[
    "apple", "bear", "bird", "boat", "book", "cake", "cat", "cloud",
    "coast", "dance", "dawn", "deer", "dove", "dream", "drum", "dusk",
    "eagle", "earth", "feast", "fire", "fish", "flame", "flower", "forest",
    "frost", "garden", "gem", "glade", "grain", "grove", "hawk", "heart",
    "hill", "lake", "leaf", "light", "moon", "mountain", "nest", "ocean",
    "orchard", "peak", "pond", "rain", "river", "rose", "shore", "sky",
    "snow", "spark", "star", "stone", "stream", "sun", "tree", "valley",
    "wave", "wind", "wood", "meadow", "willow", "tangerine", "expedition",
    "transformation", "symphony", "journey", "voyage", "odyssey", "quest",
];

const VERBS: &[&str] = &[
    "rise", "run", "sing", "dance", "glow", "bloom", "shine", "soar",
    "spark", "swift", "bright", "clear", "drift", "float", "flow",
    "leap", "spring", "sweep", "turn", "weave", "whirl", "wing",
];

/// Generate a random 3-word hyphenated name
pub fn generate_plan_name() -> String {
    let mut rng = rand::rng();

    let adjective = ADJECTIVES[rng.random_range(0..ADJECTIVES.len())];
    let noun = NOUNS[rng.random_range(0..NOUNS.len())];
    let use_verb = rng.random_bool(0.5);
    let verb_or_noun = if use_verb {
        VERBS[rng.random_range(0..VERBS.len())]
    } else {
        NOUNS[rng.random_range(0..NOUNS.len())]
    };

    format!("{}-{}-{}", adjective, noun, verb_or_noun)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_plan_name_format() {
        let name = generate_plan_name();
        let parts: Vec<&str> = name.split('-').collect();
        assert_eq!(parts.len(), 3);
        assert!(parts.iter().all(|p| !p.is_empty()));
    }

    #[test]
    fn test_generate_plan_name_uniqueness() {
        let names: std::collections::HashSet<String> = (0..100)
            .map(|_| generate_plan_name())
            .collect();
        // At least 90 unique names out of 100
        assert!(names.len() > 90);
    }
}
