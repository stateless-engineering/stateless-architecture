//! Mesh sync protocol stub.
//!
//! Placeholder for the CRDT-based peer-to-peer synchronization layer.
//! The full implementation will live in stateless-platform.

/// A CRDT operation envelope (mirrors the protobuf Operation message).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Operation {
    pub op_type: OpType,
    pub target: String,
    pub value: Option<serde_json::Value>,
    pub lamport: u64,
    pub origin: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum OpType {
    Insert,
    Delete,
    Update,
    Move,
}

/// A mesh peer — can send and receive operations.
pub trait MeshPeer {
    fn send(&mut self, op: Operation);
    fn receive(&mut self) -> Option<Operation>;
}

/// A simple in-memory mesh (for testing).
pub struct MockMesh {
    pub log: Vec<Operation>,
}

impl MockMesh {
    pub fn new() -> Self {
        Self { log: Vec::new() }
    }

    pub fn broadcast(&mut self, op: Operation) {
        self.log.push(op);
    }
}

impl Default for MockMesh {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mock_mesh_broadcasts() {
        let mut mesh = MockMesh::new();
        mesh.broadcast(Operation {
            op_type: OpType::Insert,
            target: "/doc/title".into(),
            value: Some(serde_json::json!("Hello")),
            lamport: 1,
            origin: "peer-1".into(),
        });
        assert_eq!(mesh.log.len(), 1);
    }
}
