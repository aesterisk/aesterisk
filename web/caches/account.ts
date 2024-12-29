import { auth } from "@/lib/auth";
import { getAccountOrCreate } from "@/types/account";
import { unstable_cacheLife as cacheLife, unstable_cacheTag as cacheTag } from "next/cache";

const getCachedAccount = async(
	ghId: string,
	email: string,
	firstName: string,
	lastName: string | null,
	image: string | null,
) => {
	"use cache";
	cacheLife("hours");
	cacheTag(`account-${ghId}`);

	return await getAccountOrCreate(ghId, email, firstName, lastName, image);
};

export const getAccount = async() => {
	const session = await auth();
	if(!session) return null;

	const { user } = session;
	if(!user) return null;

	const [firstName, lastName] = [...(user.name?.split(" ") ?? ["AesteriskNoFirstName", "AesteriskNoLastName"]), null];

	return await getCachedAccount(user.gh, user.email as string, firstName!, lastName, user.image ?? null);
};
