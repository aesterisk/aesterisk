"use client";

import Link from "next/link";
import { ColumnDef } from "@tanstack/react-table";

import { Node } from "@/lib/types/node";
import { cn, mapValue } from "@/lib/utils";
import { Button } from "@/components/ui/button";
import { Progress } from "@/components/ui/progress";

export const columns: ColumnDef<Node>[] = [
	{
		id: "name",
		header: () => <span className="w-min h-min select">{ "Node" }</span>,
		cell: ({ row }) => (
			// todo: link to /panel/[teamID]/nodes/[nodeID]
			<Link href="" className="select">{ row.original.name }</Link>
		),
	},
	{
		id: "status",
		header: () => <span className="w-min h-min select">{ "Status" }</span>,
		cell: ({ row }) => {
			const online = row.original.lastActive * 1000 > Date.now() - 60_000_000;

			return (
				<div className="flex items-center gap-2">
					<div className={cn("rounded-full bg-rose-600 w-2 h-2", online && "bg-emerald-600")} />
					<span>{ online ? "Online" : "Offline" }</span>
				</div>
			);
		},
		minSize: 1,
		size: 1,
	},
	{
		id: "servers",
		header: () => <span className="w-min h-min select">{ "Servers" }</span>,
		cell: ({ row }) => {
			const online = 3;
			const failed = 1;
			const offline = 6;

			return (
				<div className="flex flex-row items-center gap-4">
					{
						online > 0 && (
							<div className="flex flex-row items-center gap-2">
								<div className="rounded-full bg-emerald-600 w-2 h-2" />
								<span>{ online }</span>
							</div>
						)
					}
					{
						failed > 0 && (
							<div className="flex flex-row items-center gap-2">
								<div className="rounded-full bg-rose-600 w-2 h-2" />
								<span>{ failed }</span>
							</div>
						)
					}
					{
						offline > 0 && (
							<div className="flex flex-row items-center gap-2">
								<div className="rounded-full bg-primary/20 w-2 h-2" />
								<span>{ offline }</span>
							</div>
						)
					}
				</div>
			);
		},
		minSize: 1,
		size: 1,
	},
	{
		id: "memory",
		header: () => <span className="w-min h-min select">{ "Memory" }</span>,
		cell: ({ row }) => {
			const usage = 13.2;
			const min = 0;
			const max = 16;

			const percentage = mapValue(usage, min, max, 0, 100);

			return (
				<div className="flex flex-col">
					<span className="text-sm text-primary/50">{ `${usage} of ${max} GB used` }</span>
					<Progress value={percentage} aria-label={`${percentage.toFixed(0)}% usage`} className="h-3 border" />
				</div>
			);
		},
	},
	{
		id: "cpu",
		header: () => <span className="w-min h-min select">{ "CPU" }</span>,
		cell: ({ row }) => {
			const usage = 87.3;

			return (
				<div className="flex flex-col">
					<span className="text-sm text-primary/50">{ `${usage.toFixed(1)}% used` }</span>
					<Progress value={usage} aria-label={`${usage.toFixed(0)}% usage`} className="h-3 border" />
				</div>
			);
		},
	},
	{
		id: "storage",
		header: () => <span className="w-min h-min select">{ "Storage" }</span>,
		cell: ({ row }) => {
			const usage = 241.9;
			const min = 0;
			const max = 256;

			const percentage = mapValue(usage, min, max, 0, 100);

			return (
				<div className="flex flex-col">
					<span className="text-sm text-primary/50">{ `${usage} of ${max} GB used` }</span>
					<Progress value={percentage} aria-label={`${percentage.toFixed(0)}% usage`} className="h-3 border" />
				</div>
			);
		},
	},
	//
	// {
	// id: "id",
	// header: () => <span className="w-min h-min select">{ "ID" }</span>,
	// cell: ({ row }) => {
	// // eslint-disable-next-line react-hooks/rules-of-hooks
	// const copy = useCallback(() => {
	// window.navigator.clipboard.writeText(row.original.id.toString(10)).then(() => {
	// 				toast.info("Copied node ID", {
	// 					description: "Successfully copied the node ID",
	// 					duration: 3000,
	// 					action: {
	// 						label: "Dismiss",
	// 						onClick: () => {},
	// 					},
	// 				});
	// }).catch(() => {
	// 				toast.error("Could not copy ID", {
	// 					description: "An error occured while copying the node ID",
	// 					duration: 3000,
	// 					action: {
	// 						label: "Dismiss",
	// 						onClick: () => {},
	// 					},
	// 				});
	// });
	// }, [row.original.id]);
	//
	// return (
	// <span className="group flex flex-row items-center gap-2">
	// 				<span className="select">{ row.original.id }</span>
	// 				<Button variant="ghost" className="h-8 w-8 p-0 opacity-0 pointer-events-none group-hover:opacity-100 group-hover:pointer-events-auto transition-opacity duration-50" onClick={copy}>
	// 					<span className="sr-only">{ "Copy Node ID" }</span>
	// 					<Copy className="h-4 w-4" />
	// 				</Button>
	// </span>
	// );
	// },
	// },
	//
	{
		id: "actions",
		cell: ({ row }) =>
		// todo: link to /panel/[teamID]/nodes/[nodeID]

			 (
				<Button size="sm" variant="outline">{ "Manage" }</Button>
			),
		minSize: 1,
		size: 1,
	},
];
