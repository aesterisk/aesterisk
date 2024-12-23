-- these are made for development purposes
-- only, and should *only* be executed
-- after the migration script has finished,
-- and the first account has been created.

INSERT INTO aesterisk.teams (
	team_path,
	team_name,
	team_plan,
	team_is_personal,
	team_created_at
) VALUES (
	'monsters',
	'Monsters Inc.',
	1,
	false,
	CURRENT_TIMESTAMP
);

INSERT INTO aesterisk.teams (
	team_path,
	team_name,
	team_plan,
	team_is_personal,
	team_created_at
) VALUES (
	'acme',
	'ACME Corp.',
	2,
	false,
	CURRENT_TIMESTAMP
);

INSERT INTO aesterisk.teams (
	team_path,
	team_name,
	team_plan,
	team_is_personal,
	team_created_at
) VALUES (
	'aperture',
	'Aperture Science',
	3,
	false,
	CURRENT_TIMESTAMP
);

INSERT INTO aesterisk.users (
	user_account,
	user_team,
	user_joined_at,
	user_owner,
	user_public_key,
	user_private_key
) VALUES (
	1,
	2,
	CURRENT_TIMESTAMP,
	true,
'-----BEGIN PUBLIC KEY-----
MIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEAxBQbQlqrJ6kArocIBgKu
+V+gUQ6Mz5hY0fKRg4hPSLgwaEZatLyzXa8n8e154+qw3f3duC0pwxSV4V3Rriis
pnl75YrGwAsNyxUT5+al0boglUhfAK50/SsFxVhMScJN0lu70c+dwMUjTn3udHZz
5Oavbmj/TzqcWDY5iYL0eadPZH6OLCGw2Lb4xvu/KM5KrT4SehYqqG7J4j3DH9Gm
23j48FGLyF1W064tGqPiZC9GhbF59wByrYMKgOwK7ZlrH1esyCcq7D/Vklcd9UOR
KQvHFMs4Ti/djBNywATTowCv8XQ0HUzqMixFdHqFPiVjfH1Yf18XzXDustG/Jx5l
GwIDAQAB
-----END PUBLIC KEY-----
',
'-----BEGIN PRIVATE KEY-----
MIIEvgIBADANBgkqhkiG9w0BAQEFAASCBKgwggSkAgEAAoIBAQDEFBtCWqsnqQCu
hwgGAq75X6BRDozPmFjR8pGDiE9IuDBoRlq0vLNdryfx7Xnj6rDd/d24LSnDFJXh
XdGuKKymeXvlisbACw3LFRPn5qXRuiCVSF8ArnT9KwXFWExJwk3SW7vRz53AxSNO
fe50dnPk5q9uaP9POpxYNjmJgvR5p09kfo4sIbDYtvjG+78ozkqtPhJ6Fiqobsni
PcMf0abbePjwUYvIXVbTri0ao+JkL0aFsXn3AHKtgwqA7ArtmWsfV6zIJyrsP9WS
Vx31Q5EpC8cUyzhOL92ME3LABNOjAK/xdDQdTOoyLEV0eoU+JWN8fVh/XxfNcO6y
0b8nHmUbAgMBAAECggEAGtg33S9FpIHmVn6lMhF2/CxD6iUIUTml22SO2Ie2dxnE
gtoy+/Cjy+05lk0xdWtBuLrTeq4wPAWa+YZbOTFXZdlXBZeo200IH+gsWAEDbcHC
ST3lx9WarbiQqzKKC0UfW2/0uGZoziYPTeU+2tIGzu1oUkDsx1+aXRdbbECyEN+C
SpujcEtZ4EpxbwxHDI26vNUgQahcEYk7b2c1fL0Xmjpjx9jHUkR1U5rVQljVLAbk
/nUCWoejrkLisE8auqtgyJhtwmHCXpDq/y473u0GBObjOcScku6qkG4PDIUQmXwE
vXFukfC5HRMYyKlIiHW0a0I5jUP5cYu1f8PUtaHqdQKBgQDkc3K87Rw7ZLwV8ymw
xUobDlk9c+nvIt08K0Eh43DsXOrqoB5nWKJ1bl84XWrJw72A72//RNrxcrcqhu49
iK2A88Xo+LjlPwOqiV7xAZC/RicBKZQvuBgh030VBDKXs7lRelvGUc4WCYTN1G2A
cgshRWGzVwXpAWsQm20n0Ru2fQKBgQDbuUh5adpxSIu4bf/YwQyhVOpOwq2LrzAI
hnqdRX3Oh1fGNQWO6keU25w5OWPZrPtvslv9lWat5hWH3YXTZG72Ob5ZhWeo8XXQ
gFp61rahYnm4BPvSFvLtdpNxN/HfTivt8AWVV+fLiKSPpDbtGEZ1JS+faKTFeM5b
pUHMLXaldwKBgQCe4JNpRNWfkL0l3sidwXd6PY5eqKCGyQ5nbAWOFelQ2IYfyl+a
rA+/75SVVJKR42mFo/+V3kMOLCOqldBWRxmzVtXK4j9kX2CjV4oJvnb2L0mtC0ed
sEBINhcFaLfuvPUi7x+oWvgiZ1hO9W96JtYKu3/pA5p+o5fcItvVFx5Y2QKBgBpI
8AKWRyuGIjDN9+1WKsyh++WwJFKxgm2CqOhnh8VNK5LqhN+z5xxHqUivNOK1bt7N
13NejAoBnFHEjl3bhequvba33s63FAD9WdWYGgD0Zts8vWNEm4sMxFam+qhcEaVF
MWXMPk566jTE0E1MuiEJcckAG7YD3avY+SYTyj3BAoGBAKTPoXjQPhOYPE7BnVCO
CVtrWwDbLgUp/iwSH4S8KZJaELy6kl8rC4yITlB9P8sTBUf1u0kW9LkySMlAN1Iy
kq4+PZ3Avtb+tHhQ1f7CKtiAxXEROWOyxNvUqudHq1wrZfMKiH2G/ixr1IPTpa6Y
lYaeQXTCWTNWM26+unkpgLTi
-----END PRIVATE KEY-----
'
);

INSERT INTO aesterisk.users (
	user_account,
	user_team,
	user_joined_at,
	user_owner,
	user_public_key,
	user_private_key
) VALUES (
	1,
	3,
	CURRENT_TIMESTAMP,
	true,
'-----BEGIN PUBLIC KEY-----
MIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEArKsoCHgP5OAIhfYMWxRu
YahtuWdpCnKekvIZpMkNxjCE2eVHbOts8f33u7UGyP06OtssYjHiSdeTOExh4Qqd
U3JpogvB6/Qpp2xIPLSKFHaCOvkhL3DqAoq+1Y8QgIyx21MJrvipTDoiv994tm0s
SGdcKHxxWRAUVQAFQ07dQQsgerV8/4NbweICzFJjmq/CZo2Qr4V+VxPDhgN0N/Zk
l+kvAih1d2NpGlnVd+HpfaSqXN5qXRY7TkB6pd1euHK7/NKO8qaIRwmG+/fkBU3l
ROsIZ0vratHdL9BoIcWq5q/bMxx2vyk3Rt7l6JSCUCxlpi0ztXL+nrsQUwIintVB
kQIDAQAB
-----END PUBLIC KEY-----
',
'-----BEGIN PRIVATE KEY-----
MIIEvAIBADANBgkqhkiG9w0BAQEFAASCBKYwggSiAgEAAoIBAQCsqygIeA/k4AiF
9gxbFG5hqG25Z2kKcp6S8hmkyQ3GMITZ5Uds62zx/fe7tQbI/To62yxiMeJJ15M4
TGHhCp1TcmmiC8Hr9CmnbEg8tIoUdoI6+SEvcOoCir7VjxCAjLHbUwmu+KlMOiK/
33i2bSxIZ1wofHFZEBRVAAVDTt1BCyB6tXz/g1vB4gLMUmOar8JmjZCvhX5XE8OG
A3Q39mSX6S8CKHV3Y2kaWdV34el9pKpc3mpdFjtOQHql3V64crv80o7ypohHCYb7
9+QFTeVE6whnS+tq0d0v0Gghxarmr9szHHa/KTdG3uXolIJQLGWmLTO1cv6euxBT
AiKe1UGRAgMBAAECggEAPO1D+gELcrJOr55La9QAwvrghLxlhyc8pxNAUzISZy11
o0FQ+7Dyx0zbHmPZGhCofF4cAiYh9/ZWX1Ysb3EEZI6JkYFekvibvhTqRGlcE65h
c5e2yvunxu/YOJj7tLBwjbh2QURC0L/uxQ+Ak7ZgGshF749Bm7CdzARu1vo7/hfu
YtBbW3B9Aa/3iRnp9ctz6TavECju9hx3eDHZce3vYmJgqyNVVgA281rqucwoiJPG
AEimqkd+SPdaCKGYqOgVClbLYpqx52FM5Vdyrgl6PFbEW6rIFDPEnXi92kQsHFuA
qFP1yYrR5oIa0nktyznNzyOW/N+ninUJsn5+DIQxxQKBgQDveuTpUpkhRMAAKcbM
6fA70q3iApSKMuXQy0EvVT5fkFvf6IeBnIFNvs8NXeleqBumEmUJ5fOWc55SQDCt
hSgO+A1B/G2A9e9wW6ppyW8A1gR6R4/7XdFk2DlGPvCh6XNspTZaKDh6eQdCEFR4
ETlbpN2fdQvHEBFjwXSm+OXn4wKBgQC4lGcbxo4vcFBTafihT16WKxN0dW9Xc8Ue
Cki+X4sBI+qp+zf6oFZpYreVBsEQxrhE86SNKvsu6tglwzvIhRTrUUSiSXs791eG
e/GuvBtgk8aQwP5DMyP/IZBBD1LEXqt3bGeLQ72WhPpO9mqoWNYLsD0lTNmifq6W
bBEO56Ji+wKBgHWdeu2o1nJDbLTnz652Orl22FWHBOL26m3oVWRkzlRq9uR6NIsQ
jWTckJ41AFev4pxmrl71I/RBZoFo6z9dhXYMl55P1owevXEGZCS+fJkBg4N2wB0p
T5zioQaCBPbQTBMQ9SfEVzC3Xjww/vtVIkGDMCGPf7BNkOzYZ21WErwXAoGAB+OH
2J8O2qYxOK9xyesrfr2Okk6mmJVBGnyUCVbqCP1w/RzYkqShrNVkRUFJpR7pJ0FG
iiYJIEM3Q73pYzOU5k3N4iXD15dFrWDEvpQZk3IdbXhiJZsc8b2Mfcta3TuylmMS
tdgKVVGPpSpQ+qr5UvyQUHTNZG2HprQtsnZh8VsCgYB/ijvmhZPWqu5vv/hVYipc
y9zGrfrH89dSSnGNwvmTxRzFlZdhfpUWEnz7CzeyDCkJ5W0poOyudBRJFJ3aq8P7
4fMc+Hgyc6z42Gvyyw/nv3H/DbgA6Sg7TjPOpTCPfQw/GIsMkFT+R8Yd4FUz14aw
fpYYZXWWm5VRJIVnEcdztw==
-----END PRIVATE KEY-----
'
);

INSERT INTO aesterisk.users (
	user_account,
	user_team,
	user_joined_at,
	user_owner,
	user_public_key,
	user_private_key
) VALUES (
	1,
	4,
	CURRENT_TIMESTAMP,
	true,
'-----BEGIN PUBLIC KEY-----
MIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEAtld9cvGjLAUYigo05eVA
cqbu3ZwsSSbReyFy18wyaeCNlufUky0pKw0OvxpjCS7rp9lnJk70l/nfo+GwPLA4
EnBifGL4+3sFeEbJFtbzNRU+6PdiKByguQg03jfEKqdrpuBmtW8APWyWocDaZnWA
LoDFqI/9gxLwts/wBC81SPvQh42VRu8HJQu0XUDPf3vGUCphWfAsDXtENyhEDiwg
gn3NlJjTcfgfBq/2IojXwXFCA8A/i5NcgDb16Ebw/O4UxJmBH6l5W9RvnCS5igLf
Y+NHBULmnZsKyso64R57TTe90bfieriMZ1gPV5wxVvcfMi0rFiLazOivij3IuKVl
MwIDAQAB
-----END PUBLIC KEY-----
',
'-----BEGIN PRIVATE KEY-----
MIIEvAIBADANBgkqhkiG9w0BAQEFAASCBKYwggSiAgEAAoIBAQC2V31y8aMsBRiK
CjTl5UBypu7dnCxJJtF7IXLXzDJp4I2W59STLSkrDQ6/GmMJLuun2WcmTvSX+d+j
4bA8sDgScGJ8Yvj7ewV4RskW1vM1FT7o92IoHKC5CDTeN8Qqp2um4Ga1bwA9bJah
wNpmdYAugMWoj/2DEvC2z/AELzVI+9CHjZVG7wclC7RdQM9/e8ZQKmFZ8CwNe0Q3
KEQOLCCCfc2UmNNx+B8Gr/YiiNfBcUIDwD+Lk1yANvXoRvD87hTEmYEfqXlb1G+c
JLmKAt9j40cFQuadmwrKyjrhHntNN73Rt+J6uIxnWA9XnDFW9x8yLSsWItrM6K+K
Pci4pWUzAgMBAAECggEASHmz+jkDU17fJxbxeaNKn4esj9cgXx8Xymo8HHwkWaMQ
hDe3bZxYrazt/JV/YcoNjtTBxB9D5xhBhQESrLyaPPIAo1Ivhr2aKflwuixL1QCR
1cVmdW0TkRgq81yVEyMijdzJ1jm9jayYnshpxqnTfihe6CX7bNrdZLx3uSIOeug4
cZoehvUwtLIuYc6QcQrTv0NB/Mr+/fBN++KReJ9yNwF2Phlhjy0cQLJgT7QBxD/Z
H5xlyqL2kwJWVglL5GUnqwuI0Ut5tA6CrewkRWKnBYTAAd3rgJaTtPwMVISoYSFv
UcZpBYcRRErR4/ScXrhM0AEMuo0jclpNy+fahhKtiQKBgQDapHcshZhTu70z5aTy
ZK3vbwDQfwzfbqNyZUfebkENmwPB6wHYX35UzUMFcZ31la/osdlmOqGbxPL8o4oY
rSYlJVLbokbZkWmvI7kU39xdAsPpbRfbJu59gkCB4M3w420MFS5TZemYpCsgyCJn
fTT6tcKzENBjdpfUGNeTwElyHQKBgQDVfzauj2W9i3Vyrd4jiIvtcWIA47kcg83n
f2rTVCNgwRfa8P3AbNqZBO4TQQvjz0LpqecwW/QmEwbzVrnRmjcL7wFsBPvqZZfc
/unT/8oG+2EY88CoMxGn2gxePMtCY3v4dDz0Wwut3WAIpSuTtPoOeBfUtYhrfXOx
1y16Q/iTjwKBgBJh3XsqyEHR+PhSCGowuMb2qDTfWa+3V5qYlVBIKMQCEnDNV1xf
uvlaQJZGlSc+rIl92m9T4p58EonXHhcxB5H771lz7U0Bgscs07TDlX1kbCBGAcl1
HnwC5XWF9wqXtGVdqoVsfhVNSCr7aJkW18t0WKhBc6PJJr64T/emJQapAoGAWolo
Fv6YlpjdZZR173uoWzkJ5naruXvrhZBzLMsxdYZtJ1urQD7pNJrymxeqgyERryVt
9QQJtVu8RtUwV8KeNWFVqQk0C2Kp0/4GCvEeK3fO5VX5DVsa0aOWOyBs0ep+WA5z
CuzRNxn577CbmjfVEu26rWmOQIci2Pf3QTIx5+ECgYABWZqM0d9jr7KEefCyNmn1
IusELOsz7n97EEy8ABbbdgchDYeQ6XzL0r37rNAHbmfqGe4stU5HwqzGiJnBUfRN
aD/sotYMeHCTaORcOlNRHHb/XO6rIzwRN9BlqHxf5cFR8BkQisXiBkxHSzHUghhS
ET8elBrTryHEzX2pptZavA==
-----END PRIVATE KEY-----
'
);

INSERT INTO aesterisk.nodes (
	node_name,
	node_last_active_at,
	node_public_key,
	node_last_external_ip,
	node_ip_locked,
	node_uuid,
	node_network_ip_range
) VALUES (
	'On-Site',
	CURRENT_TIMESTAMP,
'-----BEGIN PUBLIC KEY-----
MIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEAyPDGY80gx7zcigNyFMvL
QqUvqrg6uEJhuYUunVc7UMlp3/Etz6wz7IxcQ5iPCEkBVfKyK+JSKxRSu5UyrFU+
+Q+GPrODJYmGoAKZgpoUCalNEE9lBWEWZGZ9XEU6vkRe3sIY7KI6+M8dEzjTxRIc
xfJ4HXMG4sXcv0OkcSrwG9i3o2X+4EqgBfpzf3Pf/aOhAd8qJRx09XbBifpz/uoN
bqM1XisUaCCXHgyxyZeF1x67GNtYtUYaPLOyz9WE9dPmNgL1f5q6QnHPISaddtSE
uNUjIHFHEmG6bBf/lBNJG/VkHnunGvbKmUv4OJSPVaRkzRDH2bvTQ6TgmsoZStfM
HQIDAQAB
-----END PUBLIC KEY-----
',
-- private key in pair:
-- -----BEGIN PRIVATE KEY-----
-- MIIEvQIBADANBgkqhkiG9w0BAQEFAASCBKcwggSjAgEAAoIBAQDI8MZjzSDHvNyK
-- A3IUy8tCpS+quDq4QmG5hS6dVztQyWnf8S3PrDPsjFxDmI8ISQFV8rIr4lIrFFK7
-- lTKsVT75D4Y+s4MliYagApmCmhQJqU0QT2UFYRZkZn1cRTq+RF7ewhjsojr4zx0T
-- ONPFEhzF8ngdcwbixdy/Q6RxKvAb2LejZf7gSqAF+nN/c9/9o6EB3yolHHT1dsGJ
-- +nP+6g1uozVeKxRoIJceDLHJl4XXHrsY21i1Rho8s7LP1YT10+Y2AvV/mrpCcc8h
-- Jp121IS41SMgcUcSYbpsF/+UE0kb9WQee6ca9sqZS/g4lI9VpGTNEMfZu9NDpOCa
-- yhlK18wdAgMBAAECggEAEe0Q9dEhJ7bpW2w7q2LghKO+uq82zD6RZn72jdWjYzo+
-- T/RXKIJUG4bZQ4DpRoCLNil+ZaKjvySDNClD3QXtDHd34kFN5ajJy53C9Iv3iUK2
-- SLDPvu9Ok33+y9JNQrFP+VvXF2OdrGayhFPhTvpy4K6x5bnf4/xpphJ/XOBkwHpB
-- 5OlPXgLHCqMCaOe6r3k9i9NYYrek0tOXHlKma6oEyoDL+8rvI938zckHc0micHm7
-- 6BUsAsKRbl2BQra+vIijhnBXbbml83h80IcmG/EEtRcl0oDjUcEehl/9gvqsp03c
-- A7N3dllbLmY0DuFPfcqgsKd4zu+hgzbuluQ7rSAfrQKBgQDob3OgJcg3rzv+O3Sf
-- cO7TpQ+o67Ck0YdEkLwfMr6as9f1HwNTJqoZFdGV/SctMp0p5bwqS1G97cvB5Z/R
-- RpIiEdFyT08UJy1evaVY0H7m66c/gHqUbo1XJlJ1APJQWodRDbE7WqPCJBWdGsjE
-- uN9XczhUtbW4VWWHzIBCL2ejVwKBgQDdT+rKpbaiJQkle3WSShofWEUzj/5pXwmU
-- D7vBNP3zs2UGKRT5PZhn691jHv+ObvT0A/guaHm5imNNkMWf7MKOXl1f2+zOBElk
-- Ug/pum95b8xs0DYOKyu/FEVQb9Q7DrdgQyQmKCquy2kOyOVoV9Pf43wIMnZ5zfCp
-- vTO9Ceo3qwKBgQC5gaHlkeHu11NpP2h/i/GARv9tkNXFZVixF1adC7Hl5F0aTCsq
-- JPSi2rAQJiArSXb3plv74WsWy3/Qe4SG0Oz2dgQUWEnDytTCBVe+v4BYqoEsBE1Q
-- w77YbERpD11VVjsjLGtj9J435va9EVBk+St/Lv0pVnD28mj67fL7X7w2NwKBgHDE
-- sgzEV8VTPc/dktER9TGXlttpOeeTR5wsUC1oxSYSeR2kfU1q92espchGGU4Id8SG
-- 6UUscyn5vBPf+vM8fv5wUv/vXkCzqnn13qnoF7k3IGEpzwF1OftJZvBPq0LUgtgI
-- HjlbKjSa7VNdRpfeeNWSYrcCj6ANMd4rzFs83B21AoGAHeG/s6Xkan6/MYO18Wx1
-- Hi4vkK4Hr8u+EkejUvsyUm+W3aWXlXx5vpwktwjVaK0RPojZbZY64UyeX+9BneLz
-- qs+oihHhtnKr7TetRgb+nrRH/YVUqEblHLfztSahzgzGxW4JMzAaMkxqZOrGPVXN
-- KgcG/0u2r7egmHRNCPGyPV4=
-- -----END PRIVATE KEY-----
	'1.2.3.4',
	false,
	gen_random_uuid(),
	2693 -- 10.3.0.0/16
);

INSERT INTO aesterisk.team_nodes VALUES (
	3,
	1
);
