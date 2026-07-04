import json
import sqlite3
from struct import unpack

db = sqlite3.connect("world_sql_content.db")
cursor = db.cursor()
cursor.execute("SELECT json FROM DestinyActivityDefinition")
activity_names = {}
for row in cursor.fetchall():
    data = json.loads(row[0])
    if (
        "displayProperties" in data
        and "name" in data["displayProperties"]
        and data["displayProperties"]["name"]
    ):
        activity_names[int(str(data["hash"]))] = data["displayProperties"]["name"]
open("activity_names.json", "w").write(json.dumps(activity_names, indent=2))

db.close()

f = open("8080718d.bin", "rb")
data = f.read()
pattern = b"\xa9\x72\x80\x80"
offsets = []
pos = 0
while True:
    pos = data.find(pattern, pos)
    if pos == -1:
        break
    offsets.append(pos)
    pos += 4

activity_to_investment = {}
for o in offsets:
    f.seek(o + 4)
    hash_val = unpack("<I", f.read(4))[0]
    f.seek(o + 4 + 0xFC)
    activity_name_hash_val = unpack("<I", f.read(4))[0]
    activity_to_investment[activity_name_hash_val] = hash_val

open("activity_to_investment.json", "w").write(
    json.dumps(activity_to_investment, indent=2)
)
