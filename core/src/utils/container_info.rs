use serde_json::json;

pub struct ContainerSize {
    pub count: usize,
    pub element_size: usize,
}

pub struct Leaf {
    pub name: String,
    pub info: ContainerSize,
}

impl Leaf {
    fn into_json(self) -> (String, serde_json::Value) {
        let fields = json!(
        {
            "count": self.info.count.to_string(),
            "size": self.info.element_size.to_string()
        });
        (self.name, fields)
    }
}

pub struct Node {
    pub name: String,
    pub children: ContainerInfo,
}

impl Node {
    fn into_json(self) -> (String, serde_json::Value) {
        let mut children = serde_json::Map::new();
        for child in self.children.0 {
            let (name, value) = child.into_json();
            children.insert(name, value);
        }
        (self.name, serde_json::Value::Object(children))
    }
}

pub enum ContainerInfoEntry {
    Leaf(Leaf),
    Node(Node),
}

impl ContainerInfoEntry {
    fn into_json(self) -> (String, serde_json::Value) {
        match self {
            ContainerInfoEntry::Leaf(leaf) => leaf.into_json(),
            ContainerInfoEntry::Node(node) => node.into_json(),
        }
    }
}

pub struct ContainerInfo(Vec<ContainerInfoEntry>);

impl ContainerInfo {
    pub fn builder() -> ContainerInfosBuilder {
        ContainerInfosBuilder(Vec::new())
    }

    pub fn into_json(self) -> serde_json::Value {
        let mut data = serde_json::Map::new();
        for entry in self.0 {
            let (name, value) = entry.into_json();
            data.insert(name, value);
        }
        serde_json::Value::Object(data)
    }
}

pub struct ContainerInfosBuilder(Vec<ContainerInfoEntry>);

impl ContainerInfosBuilder {
    pub fn leaf(mut self, name: impl Into<String>, count: usize, element_size: usize) -> Self {
        self.0.push(ContainerInfoEntry::Leaf(Leaf {
            name: name.into(),
            info: ContainerSize {
                count,
                element_size,
            },
        }));
        self
    }

    pub fn node(mut self, name: impl Into<String>, infos: ContainerInfo) -> Self {
        self.0.push(ContainerInfoEntry::Node(Node {
            name: name.into(),
            children: infos,
        }));
        self
    }
    pub fn finish(self) -> ContainerInfo {
        ContainerInfo(self.0)
    }
}

impl<const N: usize> From<[(&'static str, usize, usize); N]> for ContainerInfo {
    fn from(value: [(&'static str, usize, usize); N]) -> Self {
        let mut builder = ContainerInfo::builder();
        for (name, count, element_size) in value {
            builder = builder.leaf(name, count, element_size);
        }
        builder.finish()
    }
}
