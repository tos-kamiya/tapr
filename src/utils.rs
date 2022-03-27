pub trait TernaryOperator {
    fn q<V>(&self, if_true: V, if_false: V) -> V;
}

impl TernaryOperator for bool {
    fn q<V>(&self, if_true: V, if_false: V) -> V {
        if *self {
            if_true
        } else {
            if_false
        }
    }
}
