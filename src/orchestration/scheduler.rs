use std::collections::{
    HashMap,
    VecDeque,
};

use uuid::Uuid;

use crate::orchestration::graph::{
    ExecutionEdge,
    ExecutionGraph,
};

pub struct ExecutionScheduler;

impl ExecutionScheduler {
    pub fn ready_nodes(
        graph: &ExecutionGraph,
        completed:
            &std::collections::HashSet<
                Uuid,
            >,
    ) -> Vec<Uuid> {
        let mut incoming:
            HashMap<
                Uuid,
                usize,
            > = HashMap::new();

        for node in &graph.nodes {
            incoming.insert(
                node.id,
                0,
            );
        }

        for edge in &graph.edges {
            if !completed.contains(
                &edge.from,
            ) {
                *incoming
                    .entry(edge.to)
                    .or_default() += 1;
            }
        }

        incoming
            .into_iter()
            .filter_map(
                |(id, count)| {
                    if count == 0
                        && !completed
                            .contains(&id)
                    {
                        Some(id)
                    } else {
                        None
                    }
                },
            )
            .collect()
    }

    pub fn topological_sort(
        graph: &ExecutionGraph,
    ) -> Vec<Uuid> {
        let mut indegree:
            HashMap<
                Uuid,
                usize,
            > = HashMap::new();

        let mut adjacency:
            HashMap<
                Uuid,
                Vec<Uuid>,
            > = HashMap::new();

        for node in &graph.nodes {
            indegree.insert(
                node.id,
                0,
            );

            adjacency.insert(
                node.id,
                Vec::new(),
            );
        }

        for ExecutionEdge {
            from,
            to,
        } in &graph.edges
        {
            adjacency
                .entry(*from)
                .or_default()
                .push(*to);

            *indegree
                .entry(*to)
                .or_default() += 1;
        }

        let mut queue =
            VecDeque::new();

        for (
            node,
            degree,
        ) in &indegree
        {
            if *degree == 0 {
                queue.push_back(
                    *node,
                );
            }
        }

        let mut ordered =
            Vec::new();

        while let Some(node) =
            queue.pop_front()
        {
            ordered.push(node);

            if let Some(children) =
                adjacency.get(&node)
            {
                for child in children {
                    if let Some(
                        degree,
                    ) = indegree
                        .get_mut(child)
                    {
                        *degree -= 1;

                        if *degree == 0
                        {
                            queue
                                .push_back(
                                    *child,
                                );
                        }
                    }
                }
            }
        }

        ordered
    }
}
