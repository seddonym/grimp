#[derive(PartialEq, Eq, Hash, Debug)]
pub struct Route {
    pub heads: Vec<u32>,
    pub middle: Vec<u32>,
    pub tails: Vec<u32>,
}

#[derive(PartialEq, Eq, Hash, Debug)]
pub struct PackageDependency {
    pub importer: u32,
    pub imported: u32,
    pub routes: Vec<Route>,
}
