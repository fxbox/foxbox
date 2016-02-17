
// Utility function. A variant of `map` that stops in case of error.
pub fn map<T, F, U, E>(vec: Vec<T>, cb: F) -> Result<Vec<U>, E> where F: Fn(T) -> Result<U, E> {
    let mut result = Vec::with_capacity(vec.len());
    for val in vec {
        result.push(try!(cb(val)));
    }
    Ok(result)
}

