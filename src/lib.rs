

pub struct Buffer<'a, T> {
    input : &'a [T], // TODO can this be cow?
    index : usize,
}

impl<'a, T> Clone for Buffer<'a, T> {
    fn clone(&self) -> Self {
        Buffer { input: self.input, index: self.index }
    }
}

impl<'a, T> Buffer<'a, T> {
    pub fn new(input : &'a [T]) -> Buffer<'a, T> {
        Buffer { input, index: 0 }
    }

    pub fn or<S, E, const N : usize>(&mut self, targets : [for<'b> fn(&mut Buffer<'b, T>) -> Result<S, E>; N]) -> Result<S, Vec<E>> {
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

    pub fn option<S, E, F : FnOnce(&mut Buffer<'a, T>) -> Result<S, E>>(&mut self, f : F) -> Result<Option<S>, E> {
            let mut ops = self.clone();
            match f(&mut ops) {
                Ok(v) => {
                    self.index = ops.index;
                    Ok(Some(v))
                },
                Err(_) => Ok(None),
            }
    }

    pub fn list<S, E, F : FnMut(&mut Buffer<'a, T>) -> Result<S, E>>(&mut self, mut f : F) -> Result<Vec<S>, E> {
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

    pub fn with_rollback<S, E, F : FnOnce(&mut Buffer<'a, T>) -> Result<S, E>>(&mut self, f : F) -> Result<S, E> {
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
    fn should_get() {

    }
}