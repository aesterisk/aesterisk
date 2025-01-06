import React from "react";
import { DropdownMenuLabel } from "@/components/ui/dropdown-menu";
import { getAccount } from "@/caches/account";

export default async function AesteriskDropdownLabel() {
	const account = await getAccount();

	if(!account) return <></>;

	return (
		<DropdownMenuLabel>
			<p>{ account.lastName === null ? account.firstName : `${account.firstName} ${account.lastName}` }</p>
			<p className="font-normal opacity-50">{ account.email }</p>
		</DropdownMenuLabel>
	);
}
