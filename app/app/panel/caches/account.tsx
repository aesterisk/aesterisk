import { cache } from "react";
import { getAccountOrCreate } from "@/lib/types/account";
import { auth } from "@/lib/auth";

const getCachedAccountFromDB = cache(getAccountOrCreate);

export async function getAccountUncached() {
	const session = await auth();
	if(!session) return null;

	const { user } = session;
	if(!user) return null;

	const [firstName, lastName] = [...(user.name?.split(" ") ?? ["AesteriskNoFirstName", "AesteriskNoLastName"]), null];

	return await getCachedAccountFromDB(user.ghId, user.email as string, firstName!, lastName, user.image ?? null);
}

export const getAccount = cache(getAccountUncached);
