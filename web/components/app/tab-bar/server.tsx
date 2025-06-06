import { getAccount } from "@/caches/account";
import { Avatar, AvatarFallback, AvatarImage } from "@ui/avatar";
import { Button } from "@ui/button";
import { DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuLabel, DropdownMenuTrigger } from "@ui/dropdown-menu";
import { getPrimaryChars } from "@/lib/utils";
import { Asterisk, LogOut } from "lucide-react";
import Link from "next/link";
import { ReactNode, Suspense } from "react";
import { Skeleton } from "@ui/skeleton";
import { signOut } from "@/lib/auth";
import { AuthError } from "next-auth";
import { redirect } from "next/navigation";

export function TabBar({ children }: Readonly<{ children: ReactNode; }>) {
	return (
		<nav
			className="bg-background-100 h-12 w-screen border-b flex flex-row items-center p-4"
		>
			{ children }
		</nav>
	);
}

export function TabLogo() {
	return (
		<Link href="/" className="size-12 grid place-items-center">
			<Asterisk className="size-6 text-accent-foreground" />
		</Link>
	);
}

export function TabSpacer() {
	return (
		<div className="flex-1" />
	);
}

async function AsyncUserAvatar() {
	const account = await getAccount();
	if(!account) return <></>;

	return (
		<>
			<AvatarImage src={account.avatar} alt="Account Avatar" />
			<AvatarFallback>{ getPrimaryChars(account.lastName === null ? account.firstName : `${account.firstName} ${account.lastName}`) }</AvatarFallback>
		</>
	);
}

export function TabAccountDropdown({ children }: Readonly<{ children: ReactNode; }>) {
	return (
		<DropdownMenu>
			<DropdownMenuTrigger asChild>
				<Button variant="secondary" size="icon" className="rounded-full cursor-pointer">
					<Avatar>
						<Suspense>
							<AsyncUserAvatar />
						</Suspense>
					</Avatar>
				</Button>
			</DropdownMenuTrigger>
			<DropdownMenuContent align="end">
				{ children }
			</DropdownMenuContent>
		</DropdownMenu>
	);
}

async function AsyncUserLabel() {
	const account = await getAccount();
	if(!account) return <></>;

	return (
		<DropdownMenuLabel>
			<p className="text-foreground">{ account.lastName === null ? account.firstName : `${account.firstName} ${account.lastName}` }</p>
			<p className="font-normal text-foreground/50">{ account.email }</p>
		</DropdownMenuLabel>
	);
}

export function TabAccountDropdownLabel() {
	return (
		<Suspense
			fallback={
				(
					<DropdownMenuLabel className="py-2.5">
						<div className="mb-2"><Skeleton className="h-3 w-24" /></div>
						<Skeleton className="h-3 w-full" />
					</DropdownMenuLabel>
				)
			}
		>
			<AsyncUserLabel />
		</Suspense>
	);
}

export function TabAccountDropdownLink({ href, icon, label }: Readonly<{
	href: string;
	icon: ReactNode;
	label: string;
}>) {
	return (
		<Link href={href} className="w-full h-full">
			<DropdownMenuItem className="w-full h-full cursor-pointer">
				{ icon }
				{ label }
			</DropdownMenuItem>
		</Link>
	);
}

export function TabAccountDropdownLogout() {
	return (
		<form
			action={
				async() => {
					"use server";
					try {
						await signOut({ redirectTo: "/" });
					} catch(error) {
						if(error instanceof AuthError) {
							redirect(`/auth/error?error=${error.type}`);
						}

						throw error;
					}
				}
			}
		>
			<button type="submit" className="w-full h-full">
				<DropdownMenuItem className="text-destructive focus:text-destructive cursor-pointer">
					<LogOut className="size-4 text-destructive/50" />
					{ "Log Out" }
				</DropdownMenuItem>
			</button>
		</form>
	);
}

export { DropdownMenuSeparator as TabAccountDropdownSeparator } from "@ui/dropdown-menu";
