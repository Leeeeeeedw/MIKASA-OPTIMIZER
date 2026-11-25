import sqlite3, json, os

p = os.path.expandvars(r'%APPDATA%\com.mikasa.optimizer\mikasa-optimizer.sqlite')
c = sqlite3.connect(p)
print('db=', p)
print('candidates=', c.execute('select count(*) from candidates').fetchone()[0])
print('by sources:')
for r in c.execute('select sources, count(*) from candidates group by sources order by 2 desc limit 10'):
    print(r)
print('source_stats:')
for r in c.execute('select source,last_fetched,last_added,last_duplicates,last_invalid,last_update,status from source_stats'):
    print(r)
row = c.execute("select value from meta where key='cm_source_status'").fetchone()
print('cm_status=', json.loads(row[0]) if row else None)
print('scans:')
for r in c.execute('select scan_id,status,started_at,finished_at from scans order by rowid desc limit 5'):
    print(r)
print('results=', c.execute('select count(*) from results').fetchone()[0])
row = c.execute("select value from meta where key='app_settings'").fetchone()
print('settings=', json.loads(row[0]) if row else None)
print('latest_scans=', c.execute("select scan_id,status,tested,final_count,avg_latency,avg_speed,started_at,finished_at from scans order by rowid desc limit 2").fetchall())
for r in c.execute("select region,rank,ip,port,colo,latency,speed_mb_s,loss,stability,source,status from results order by rowid desc limit 20"):
    print('result=', r)
