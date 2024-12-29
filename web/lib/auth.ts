import NextAuth from "next-auth";
import GitHub from "next-auth/providers/github";
import env from "./env";
import "next-auth/jwt";

declare module "next-auth" {
	interface User {
		gh: string;
		mfa: boolean;
	}
}

declare module "next-auth/jwt" {
	interface JWT {
		gh: string;
		mfa: boolean;
	}
}

export const { handlers, signIn, signOut, auth } = NextAuth({
	providers: [
		GitHub({
			clientId: env.private.GITHUB_CLIENT_ID,
			clientSecret: env.private.GITHUB_CLIENT_SECRET,
			profile(profile) {
				return {
					email: profile.email,
					name: profile.name,
					image: profile.avatar_url,
					mfa: profile.two_factor_authentication,
					gh: profile.id.toString(10),
				};
			},
		}),
	],
	pages: {
		signIn: "/auth/login",
		error: "/auth/error",
	},
	callbacks: {
		jwt({ token, user }) {
			if(user) {
				token.gh = user.gh;
				token.mfa = user.mfa;
			}

			return token;
		},
		async session({ session, token }) {
			session.user.gh = token.gh;
			session.user.mfa = token.mfa;

			return session;
		},
	},
});
