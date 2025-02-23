"use client";

import Link from "next/link";
import { ColumnDef } from "@tanstack/react-table";
import { Button } from "@/components/ui/button";
import { NetworkData } from ".";

export const columns: ColumnDef<NetworkData>[] = [
	{
		id: "name",
		header: () => <span className="w-min h-min select">{ "Network" }</span>,
		cell: ({ row }) => (
			// todo: link to /dash/[teamID]/networks/[networkID]
			<Link href="" className="select">{ row.original.name }</Link>
		),
		size: 300,
		minSize: 300,
	},
	{
		id: "node",
		header: () => <span className="w-min h-min select">{ "On Node" }</span>,
		cell: ({ row }) => (
			// todo: link to /dash/[teamID]/nodes/[nodeID]
			<Link href="" className="select">{ row.original.node.name }</Link>
		),
		size: 300,
		minSize: 300,
	},
	{
		id: "localIp",
		header: () => <span className="w-min h-min select">{ "Subnet" }</span>,
		cell: ({ row }) => (
			// todo: change 10.133 to a user-defined value on the node
			<code>{ `10.133.${row.original.localIp}.0/24` }</code>
		),
		size: 1000,
		minSize: 1000,
	},
	{
		id: "actions",
		cell: ({ row }) => (
			// todo: link to /dash/[teamID]/networks/[networkID]
			// todo: or a ... menu with options such as copy the docker id etc
			<Button size="sm" variant="outline">{ "Manage" }</Button>
		),
		minSize: 1,
		size: 1,
	},
];
