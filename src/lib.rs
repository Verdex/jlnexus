
use std::rc::Rc;
use std::ops::Deref;

enum Input<'a, T> {
    Ref(&'a [T]),
    Rc(Rc<[T]>),
}

impl<'a, T> Clone for Input<'a, T> {
    fn clone(&self) -> Self {
        match self {
            Input::Ref(x) => Input::Ref(x),
            Input::Rc(x) => Input::Rc(Rc::clone(x)),
        }
    }
}

impl<'a, T> Deref for Input<'a, T> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        match self {
            Input::Ref(x) => x,
            Input::Rc(x) => &*x,
        }
    }
}

pub struct Parser<'a, T> {
    input : Input<'a, T>,
    index : usize,
}

impl<'a, T> From<&'a [T]> for Parser<'a, T> {
    fn from(item : &'a [T]) -> Self {
        Parser { input: Input::Ref(item), index: 0 }
    }
}

impl<'a, T> From<Vec<T>> for Parser<'a, T> {
    fn from(item : Vec<T>) -> Self {
        Parser { input: Input::Rc(item.into()), index: 0 }
    }
}

impl<'a, T> From<&Rc<[T]>> for Parser<'a, T> {
    fn from(item : &Rc<[T]>) -> Self {
        Parser { input: Input::Rc(Rc::clone(item)), index: 0 }
    }
}

impl<'a, T> FromIterator<T> for Parser<'a, T> {
    fn from_iter<S>(iter : S) -> Self where S : IntoIterator<Item = T> {
        iter.into_iter().collect::<Vec<_>>().into()
    }
}

impl<'a, T> Clone for Parser<'a, T> {
    fn clone(&self) -> Self {
        Parser { input: self.input.clone(), index: self.index }
    }
}

impl<'a, T> Parser<'a, T> {
    pub fn new(input : &'a [T]) -> Parser<'a, T> {
        Parser { input: Input::Ref(input), index: 0 }
    }

    pub fn or<S, E, const N : usize>(&mut self, targets : [for<'b> fn(&mut Parser<'b, T>) -> Result<S, E>; N]) -> Result<S, Vec<E>> {
        let mut errors = vec![];
        for target in targets {
            let mut ops = self.clone();
            match target(&mut ops) {
                Ok(s) => { 
                    self.index = ops.index;
                    return Ok(s); 
                },
                Err(e) => { errors.push(e); },
            }
        }

        Err(errors)
    }

    pub fn option<S, E, F : FnOnce(&mut Parser<'a, T>) -> Result<S, E>>(&mut self, f : F) -> Result<Option<S>, E> {
            let mut ops = self.clone();
            match f(&mut ops) {
                Ok(v) => {
                    self.index = ops.index;
                    Ok(Some(v))
                },
                Err(_) => Ok(None),
            }
    }

    pub fn list<S, E, F : FnMut(&mut Parser<'a, T>) -> Result<S, E>>(&mut self, mut f : F) -> Result<Vec<S>, E> {
        let mut rets = vec![];
        loop {
            let mut ops = self.clone();
            match f(&mut ops) {
                Ok(v) => {
                    self.index = ops.index;
                    rets.push(v);
                },
                Err(_) => { break; },
            }
        }
        Ok(rets)
    }

    pub fn peek<E>(&self, e : E) -> Result<&T, E> {
        if self.index < self.input.len() {
            let r = &self.input[self.index];
            Ok(r)
        }
        else {
            Err(e)
        }
    }

    pub fn get<E>(&mut self, e : E) -> Result<&T, E> {
        if self.index < self.input.len() {
            let r = &self.input[self.index];
            self.index += 1;
            Ok(r)
        }
        else {
            Err(e)
        }
    }

    pub fn end(&self) -> bool {
        self.index >= self.input.len()
    }

    pub fn index(&self) -> usize {
        self.index
    }

    pub fn with_rollback<S, E, F : FnOnce(&mut Parser<'a, T>) -> Result<S, E>>(&mut self, f : F) -> Result<S, E> {
        let mut ops = self.clone();
        let r = f(&mut ops)?;
        self.index = ops.index;
        Ok(r)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn should_create_borrow_buffer_with_into() {
        let input = vec![1, 2, 3];
        let mut buffer : Parser<usize> = (&input[..]).into();

        let value = buffer.get(()).unwrap();

        assert_eq!(*value, 1);
        assert_eq!(buffer.index(), 1)
    }

    #[test]
    fn should_get() {
        let input = vec![1, 2, 3];
        let mut buffer = Parser::new(&input);

        let value = buffer.get(()).unwrap();

        assert_eq!(*value, 1);
        assert_eq!(buffer.index(), 1)
    }

    #[test]
    fn should_peek() {
        let input = vec![1, 2, 3];
        let buffer = Parser::new(&input);

        let value = buffer.peek(()).unwrap();

        assert_eq!(*value, 1);
        assert_eq!(buffer.index(), 0);
    }

    #[test]
    fn should_rollback() {
        let input = vec![1, 2, 3];
        let mut buffer = Parser::new(&input);

        let _ = buffer.with_rollback(|buffer| {
            buffer.get(())?;
            Err::<usize, ()>(())
        });

        assert_eq!(buffer.index(), 0);
    }

    #[test]
    fn should_indicate_end() {
        let input = vec![1, 2, 3];
        let mut buffer = Parser::new(&input);

        assert!(!buffer.end());
        buffer.get(()).unwrap();
        assert!(!buffer.end());
        buffer.get(()).unwrap();
        assert!(!buffer.end());
        buffer.get(()).unwrap();
        assert!(buffer.end());
    }

    #[test]
    fn should_get_option() {
        let input = vec![1, 2, 3];
        let mut buffer = Parser::new(&input);

        let result = buffer.option(|_| Err::<usize, ()>(())).unwrap();
        assert!(result.is_none());
        assert_eq!(buffer.index(), 0);

        let result = buffer.option(|buffer| Ok::<usize, ()>(*buffer.get(())?)).unwrap();
        assert!(matches!(result, Some(1)));
        assert_eq!(buffer.index(), 1);
    }

    #[test]
    fn should_get_list() {
        let input = vec![1, 2, 3];
        let mut buffer = Parser::new(&input);

        let result = buffer.list(|buffer| Ok::<usize, ()>(*buffer.get(())?)).unwrap();

        assert_eq!(result, vec![1, 2, 3]);
    }

    #[test]
    fn should_get_or() {
        fn even(input : &mut Parser<usize>) -> Result<bool, ()> {
            if input.get(())? % 2 == 0 {
                Ok(true)
            }
            else {
                Err(())
            }
        }

        fn odd(input : &mut Parser<usize>) -> Result<bool, ()> {
            if input.get(())? % 2 == 1 {
                Ok(false)
            }
            else {
                Err(())
            }
        }
        
        let input = vec![1, 2, 3];
        let mut buffer = Parser::new(&input);

        let result = buffer.list(|buffer| buffer.or([even, odd])).unwrap();

        assert_eq!(result, vec![false, true, false]);
    }
}