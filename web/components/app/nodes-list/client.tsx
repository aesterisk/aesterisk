"use client";

import { Node } from "@/types/node";
import { DataList } from "../data-list";
import { columns } from "./columns";
import { NodeData } from ".";
import { useCallback, useMemo, useState } from "react";
import useEvent from "@/hooks/event";
import { EventOf, EventType } from "@/packets/events";

export default function Client({ nodes }: { nodes: Node[]; }) {
	const [nodeData, setNodeData] = useState<NodeData[]>(nodes.map((node) => ({
		uuid: node.uuid,
		name: node.name,
		lastActive: node.lastActive,
	})));

	const nodeUuids = useMemo(() => nodes.map((node) => node.uuid), [nodes]);

	const updateStatus = useCallback((event: EventOf<EventType.NodeStatus>) => {
		setNodeData((data) => data.map((node) => ({
			...node,
			online: node.uuid === event.daemon ? event.event.NodeStatus.online : node.online,
		})));
	}, []);

	useEvent(EventType.NodeStatus, updateStatus, nodeUuids);

	return (
		<DataList columns={columns} data={nodeData} />
	);
}
