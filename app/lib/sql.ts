import "server-only";

import { sql as vercelSql } from "@vercel/postgres";
import { NeonQueryFunction, neon, neonConfig } from "@neondatabase/serverless";

let neonSql: NeonQueryFunction<false, false> | null = null;

if(process.env.VERCEL_ENV === "development") {
	neonConfig.fetchEndpoint = "http://db.localtest.me:54331/sql";

	neonSql = neon(process.env.POSTGRES_URL!);
}

type Primitive = string | number | boolean | undefined | null;

let dbCalls = 0;

export const sql = async(
	strings: TemplateStringsArray,
	...values: Primitive[]
): Promise<Record<string, unknown>[]> => {
	dbCalls++;
	console.warn("(SQL)", strings.join(""));

	if(neonSql) return await neonSql(strings, ...values);
	return (await vercelSql(strings, ...values)).rows;
};

export const resetDatabaseMetrics = () => {
	dbCalls = 0;
};

export const getDatabaseMetrics = () => dbCalls;
