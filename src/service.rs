use crate::pods::pod::Pod;
use std::collections::HashMap;

pub struct Service {
    pods: HashMap<String, Pod>,
    socket: String,
}
