"use client";

import { ColumnDef, flexRender, getCoreRowModel, getPaginationRowModel, useReactTable } from "@tanstack/react-table";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { Button } from "@/components/ui/button";
import { useEffect } from "react";
import { eventsBus } from "@/lib/buses/events";
import { Node } from "@/lib/types/node";

interface DataTableProps<TValue> {
	columns: ColumnDef<Node, TValue>[];
	data: Node[];
}

export function DataTable<TValue>({ columns, data }: DataTableProps<TValue>) {
	useEffect(() => {
		const unsubscribe = eventsBus.on("NodesList", (nodes) => {
			console.log("Got node status", nodes);
		}, data.map((node) => node.id));

		return () => {
			unsubscribe();
		};
	}, [data]);

	const table = useReactTable({
		data,
		columns,
		getCoreRowModel: getCoreRowModel(),
		getPaginationRowModel: getPaginationRowModel(),
	});

	return (
		<div>
			<div className="rounded-md border">
				<Table>
					<TableHeader>
						{
							table.getHeaderGroups().map((headerGroup) => (
								<TableRow key={headerGroup.id}>
									{
										headerGroup.headers.map((header) => (
											<TableHead
												key={header.id}
												colSpan={header.colSpan}
												style={{ width: header.getSize() }}
											>
												{
													header.isPlaceholder
														? null
														: flexRender(
															header.column.columnDef.header,
															header.getContext()
														)
												}
											</TableHead>
										))
									}
								</TableRow>
							))
						}
					</TableHeader>
					<TableBody>
						{
							table.getRowModel().rows?.length
								? (
									table.getRowModel().rows.map((row) => (
										<TableRow key={row.id} data-state={row.getIsSelected() && "selected"}>
											{
												row.getVisibleCells().map((cell) => (
													<TableCell key={cell.id} style={{ width: cell.column.getSize() }}>
														{ flexRender(cell.column.columnDef.cell, cell.getContext()) }
													</TableCell>
												))
											}
										</TableRow>
									))
								)
								: (
									<TableRow>
										<TableCell colSpan={columns.length} className="h-24 text-center">
											{ "No results!" }
										</TableCell>
									</TableRow>
								)
						}
					</TableBody>
				</Table>
			</div>
			{ /* todo: redo pagination */ }
			<div className="flex items-center justify-end space-x-2 py-4">
				<Button variant="outline" size="sm" onClick={() => table.previousPage()} disabled={!table.getCanPreviousPage()}>
					{ "Previous" }
				</Button>
				<Button variant="outline" size="sm" onClick={() => table.nextPage()} disabled={!table.getCanNextPage()}>
					{ "Next" }
				</Button>
			</div>
		</div>
	);
}
