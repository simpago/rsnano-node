use serde_json::json;

pub struct ContainerSize {
    pub count: usize,
    pub element_size: usize,
}

pub struct Leaf {
    pub name: String,
    pub info: ContainerSize,
}

pub struct Node {
    pub name: String,
    pub children: ContainerInfos,
}

pub enum ContainerInfoEntry {
    Leaf(Leaf),
    Node(Node),
}

impl From<ContainerInfoEntry> for ContainerInfoComponent {
    fn from(value: ContainerInfoEntry) -> Self {
        match value {
            ContainerInfoEntry::Leaf(leaf) => ContainerInfoComponent::Leaf(ContainerInfo {
                name: leaf.name,
                count: leaf.info.count,
                sizeof_element: leaf.info.element_size,
            }),
            ContainerInfoEntry::Node(mut node) => ContainerInfoComponent::Composite(
                node.name,
                node.children.0.drain(..).map(|c| c.into()).collect(),
            ),
        }
    }
}

pub struct ContainerInfos(Vec<ContainerInfoEntry>);

impl ContainerInfos {
    pub fn builder() -> ContainerInfosBuilder {
        ContainerInfosBuilder(Vec::new())
    }

    pub fn into_legacy(mut self, name: impl Into<String>) -> ContainerInfoComponent {
        let entries: Vec<ContainerInfoComponent> = self.0.drain(..).map(|i| i.into()).collect();
        ContainerInfoComponent::Composite(name.into(), entries)
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

    pub fn node(mut self, name: impl Into<String>, infos: ContainerInfos) -> Self {
        self.0.push(ContainerInfoEntry::Node(Node {
            name: name.into(),
            children: infos,
        }));
        self
    }
    pub fn finish(self) -> ContainerInfos {
        ContainerInfos(self.0)
    }
}

impl<const N: usize> From<[(&'static str, usize, usize); N]> for ContainerInfos {
    fn from(value: [(&'static str, usize, usize); N]) -> Self {
        let mut builder = ContainerInfos::builder();
        for (name, count, element_size) in value {
            builder = builder.leaf(name, count, element_size);
        }
        builder.finish()
    }
}

#[derive(Clone)]
pub struct ContainerInfo {
    pub name: String,
    pub count: usize,
    pub sizeof_element: usize,
}

#[derive(Clone)]
pub enum ContainerInfoComponent {
    Leaf(ContainerInfo),
    Composite(String, Vec<ContainerInfoComponent>),
}

impl ContainerInfoComponent {
    pub fn into_json(self) -> serde_json::Value {
        let (key, value) = self.into_json_impl();
        let mut data = serde_json::Map::new();
        data.insert(key, value);
        serde_json::Value::Object(data)
    }

    fn into_json_impl(self) -> (String, serde_json::Value) {
        match self {
            ContainerInfoComponent::Leaf(leaf) => {
                let fields = json!({"count": leaf.count.to_string(), "size": leaf.sizeof_element.to_string()});
                (leaf.name.clone(), fields)
            }
            ContainerInfoComponent::Composite(name, leafs) => {
                let mut leaf_trees = serde_json::Map::new();
                for leaf in leafs {
                    let (name, fields) = leaf.into_json_impl();
                    leaf_trees.insert(name, fields);
                }

                (name.clone(), serde_json::Value::Object(leaf_trees))
            }
        }
    }
}
