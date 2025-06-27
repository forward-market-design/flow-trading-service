GET {{baseurl}}/docs/api.json
HTTP 200

[Asserts]
header "Content-Type" contains "application/json"
