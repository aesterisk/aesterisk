export { auth as middleware } from "@/lib/auth";

export const config = {
	matcher: ["/dash/:team*"],
};
