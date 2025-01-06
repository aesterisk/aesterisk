import path from "node:path";
import { fileURLToPath } from "node:url";
import { FlatCompat } from "@eslint/eslintrc";

const filename = fileURLToPath(import.meta.url);
const dirname = path.dirname(filename);
const compat = new FlatCompat({
	baseDirectory: dirname,
});

const eslintConfig = [
	{
		ignores: [".next/", "next-env.d.ts"],
	},
	...compat.extends("next/core-web-vitals", "next/typescript", "@yolocat"),
	{
		rules: {
			"@stylistic/no-extra-parens": "off",
			"id-length": "off",
			"no-nested-ternary": "off",
			"@typescript-eslint/no-shadow": "error",
			"no-shadow": "off",
			"@typescript-eslint/no-unused-vars": "error",
			"no-unused-vars": "off",
			"new-cap": [
				"error", {
					capIsNew: false,
				},
			],
		},
	},
];

export default eslintConfig;
