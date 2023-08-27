use self::{
    asterisk::Asterisk, comma::Comma, hyphen::Hyphen, letter::Letter, num::Number, slash::Slash,
    start::Start, table::Entry,
};
use crate::ParseError;
use std::{collections::HashMap, str::CharIndices};

enum Token {
    Start(Start),
    Number(Number),
    Letter(Letter),
    Comma(Comma),
    Hyphen(Hyphen),
    Asterisk(Asterisk),
    Slash(Slash),
    End,
}

impl Token {
    pub fn is_valid(&self, token: &Self) -> bool {
        match self {
            Token::Start(_) => Start::is_valid(token),
            Token::Number(_) => Number::is_valid(token),
            Token::Letter(_) => Letter::is_valid(token),
            Token::Comma(_) => Comma::is_valid(token),
            Token::Hyphen(_) => Hyphen::is_valid(token),
            Token::Asterisk(_) => Asterisk::is_valid(token),
            Token::Slash(_) => Slash::is_valid(token),
            Token::End => false, // Nothing can come after the end!!
        }
    }

    pub fn id(&self) -> usize {
        match self {
            Token::Start(s) => s.id(),
            Token::Number(n) => n.id(),
            Token::Letter(l) => l.id(),
            Token::Comma(c) => c.id(),
            Token::Hyphen(h) => h.id(),
            Token::Asterisk(a) => a.id(),
            Token::Slash(s) => s.id(),
            Token::End => usize::MAX,
        }
    }
}

pub struct TokenStream {
    tokens: Vec<Token>,
    table: HashMap<usize, Entry>,
    next_id: usize,
}

impl TokenStream {
    fn new() -> Self {
        Self {
            tokens: vec![Token::Start(Start::new(0))],
            table: HashMap::new(),
            next_id: 1,
        }
    }

    pub fn tokenize(chars: CharIndices) -> Result<Self, ParseError> {
        let mut stream = Self::new();
        for (pos, char) in chars {
            let id = stream.next_id();
            let token = match &char {
                '0'..='9' => Token::Number(Number::new(id)),
                'a'..='z' | 'A'..='Z' => Token::Letter(Letter::new(id)),
                ',' => Token::Comma(Comma::new(id)),
                '/' => Token::Slash(Slash::new(id)),
                '*' => Token::Asterisk(Asterisk::new(id)),
                '-' => Token::Hyphen(Hyphen::new(id)),
                _ => {
                    return Err(ParseError::InvalidToken);
                }
            };

            if stream.is_valid(&token) {
                let already_exists = stream
                    .table
                    .insert(token.id(), Entry::new(char, pos))
                    .is_some();
                if already_exists {
                    panic!("Parse table should have unique ids!");
                }
                stream.tokens.push(token);
            } else {
                return Err(ParseError::InvalidToken);
            }
        }
        let final_token = Token::End;
        if stream.is_valid(&final_token) {
            stream.tokens.push(final_token);
        } else {
            return Err(ParseError::BadEnd);
        }

        Ok(stream)
    }

    fn is_valid(&self, token: &Token) -> bool {
        let last = self
            .tokens
            .last()
            .expect("tokens should have at least the start token");
        last.is_valid(token)
    }

    fn next_id(&mut self) -> usize {
        let id = self.next_id;
        self.next_id += 1;
        id
    }
}

impl ToString for TokenStream {
    fn to_string(&self) -> String {
        let mut ret = String::with_capacity(self.tokens.len());
        for token in self.tokens.iter() {
            let char = self.table.get(&token.id());
            if let Some(entry) = char {
                ret.push(entry._value())
            }
        }

        ret
    }
}

mod table {
    #[derive(Debug)]
    pub struct Entry {
        _value: char,
        _pos: usize,
    }

    impl Entry {
        pub fn new(_value: char, _pos: usize) -> Self {
            Self { _value, _pos }
        }
    
    pub fn _value(&self) -> char {
        self._value
    }
}
}

mod start {
    use super::Token;

    pub(super) struct Start {
        id: usize,
    }

    impl Start {
        pub fn new(id: usize) -> Self {
            Self { id }
        }

        pub fn id(&self) -> usize {
            self.id
        }

        pub fn is_valid(token: &Token) -> bool {
            match token {
                Token::Start(_) => false,
                Token::Number(_) => true,
                Token::Letter(_) => true,
                Token::Comma(_) => false,
                Token::Hyphen(_) => false,
                Token::Asterisk(_) => true,
                Token::Slash(_) => false,
                Token::End => true,
            }
        }
    }
}

mod num {
    use super::Token;

    pub(super) struct Number {
        id: usize,
    }

    impl Number {
        pub fn new(id: usize) -> Self {
            Self { id }
        }

        pub fn id(&self) -> usize {
            self.id
        }

        pub fn is_valid(token: &Token) -> bool {
            match token {
                Token::Start(_) => false,
                Token::Number(_) => true,
                Token::Letter(_) => false,
                Token::Comma(_) => true,
                Token::Hyphen(_) => true,
                Token::Asterisk(_) => false,
                Token::Slash(_) => false,
                Token::End => true,
            }
        }
    }
}

mod letter {
    use super::Token;

    pub(super) struct Letter {
        id: usize,
    }

    impl Letter {
        pub fn new(id: usize) -> Self {
            Self { id }
        }

        pub fn id(&self) -> usize {
            self.id
        }

        pub fn is_valid(token: &Token) -> bool {
            match token {
                Token::Start(_) => false,
                Token::Number(_) => false,
                Token::Letter(_) => true,
                Token::Comma(_) => false,
                Token::Hyphen(_) => true,
                Token::Asterisk(_) => false,
                Token::Slash(_) => false,
                Token::End => true,
            }
        }
    }
}

mod comma {
    use super::Token;

    pub(super) struct Comma {
        id: usize,
    }

    impl Comma {
        pub fn new(id: usize) -> Self {
            Self { id }
        }

        pub fn id(&self) -> usize {
            self.id
        }

        pub fn is_valid(token: &Token) -> bool {
            matches!(token, Token::Number(_) | Token::Letter(_))
        }
    }
}

mod hyphen {
    use super::Token;

    pub(super) struct Hyphen {
        id: usize,
    }

    impl Hyphen {
        pub fn new(id: usize) -> Self {
            Self { id }
        }

        pub fn id(&self) -> usize {
            self.id
        }

        pub fn is_valid(token: &Token) -> bool {
            matches!(token, Token::Number(_) | Token::Letter(_))
        }
    }
}

mod asterisk {
    use super::Token;

    pub(super) struct Asterisk {
        id: usize,
    }

    impl Asterisk {
        pub fn new(id: usize) -> Self {
            Self { id }
        }

        pub fn id(&self) -> usize {
            self.id
        }

        pub fn is_valid(token: &Token) -> bool {
            matches!(token, Token::Slash(_) | Token::End)
        }
    }
}

mod slash {
    use super::Token;

    pub(super) struct Slash {
        id: usize,
    }

    impl Slash {
        pub fn new(id: usize) -> Self {
            Self { id }
        }

        pub fn id(&self) -> usize {
            self.id
        }

        pub fn is_valid(token: &Token) -> bool {
            matches!(token, Token::Number(_))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::TokenStream;

    #[test]
    fn can_tokenize_asterisk() {
        let stream = TokenStream::tokenize("*".char_indices());
        assert!(stream.is_ok())
    }

    #[test]
    fn can_tokenize_number() {
        let stream = TokenStream::tokenize("34".char_indices());
        assert!(stream.is_ok())
    }

    #[test]
    fn garbage_fails() {
        let stream = TokenStream::tokenize("3qd939j-3rl/;f.".char_indices());
        assert!(stream.is_err())
    }

    #[test]
    fn interval_and_multiple_works() {
        let stream = TokenStream::tokenize("8,3-3,34".char_indices());
        assert!(stream.is_ok())
    }

    #[test]
    fn interval_and_comma_fails() {
        let stream = TokenStream::tokenize("*,3/,4".char_indices());
        assert!(stream.is_err())
    }

    #[test]
    fn to_string_works_for_stream() {
        let stream = TokenStream::tokenize("4,5,9,10".char_indices()).unwrap();
        assert_eq!(stream.to_string(), "4,5,9,10")
    }
}
