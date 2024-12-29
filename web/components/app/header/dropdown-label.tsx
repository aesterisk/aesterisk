import React from "react";
import { DropdownMenuLabel } from "@/components/ui/dropdown-menu";
import { getAccount } from "@/caches/account";

export default async function AesteriskDropdownLabel() {
	const account = await getAccount();

	if(!account) return <></>;

	return (
		<DropdownMenuLabel>{ account.lastName === null ? account.firstName : `${account.firstName} ${account.lastName}` }</DropdownMenuLabel>
	);
}
