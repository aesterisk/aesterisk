import { getUserById } from "@/lib/types/user";
import { NextRequest, NextResponse } from "next/server";

export async function GET(req: NextRequest): Promise<NextResponse> {
	const params = req.nextUrl.searchParams;

	const id = params.get("id");
	const key = params.get("key");

	if(!id || !key) return new NextResponse("Invalid request", { status: 400 });

	try {
		const userId = parseInt(id, 10);
		const user = await getUserById(userId);

		if(!user) return new NextResponse("Invalid user", { status: 404 });
		if(user.publicKey !== key) return new NextResponse("Invalid key", { status: 401 });

		return new NextResponse("Valid key", { status: 200 });
	} catch(_) {
		return new NextResponse("Invalid request", { status: 400 });
	}
}

