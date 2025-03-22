#[macro_export]
macro_rules! cloned {
($($es:ident),+) => {$(
    #[allow(unused_mut)]
    let mut $es = $es.clone();
)*}
}
