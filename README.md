# gamesheet-schedule

A simple tool that will dump a league schedule from the gamesheet api to a json
file named `schedule.json` in the specified output directory.

### Example

`cargo run -- -i 1234 -t "Buffalo Team" -o /var/tmp`

If you serve this file from a webserver you can dump the schedule in a nice
format with something like `jq`

```bash
#!/bin/bash

(printf "VISITOR\tHOME\tRINK\tWHEN\n";
curl -sSf \
    https://mywebsite.com/schedule.json |
    jq -r '.games[] | [.visitor, .home, .location, .scheduled_at] | @tsv') |
    column -ts $'\t'

```

Which outputs something like:
```
VISITOR               HOME                  RINK                        WHEN
Swedish Fish          Buffalo Team          Hockey Center - Rink #1     Friday June 03 09:30:00 PM
Buffalo Team          Flaming Puke Buckets  Hockey Center - Rink #2     Friday June 10 10:00:00 PM
White/Royal           Buffalo Team          Hockey Center - Rink #1     Friday June 17 09:30:00 PM
Buffalo Team          Pitter Patter         Hockey Center - Rink #2     Friday June 24 08:15:00 PM
Bad News Blades       Buffalo Team          Hockey Center - Rink #1     Friday July 08 09:45:00 PM
Buffalo Team          Silver                Hockey Center - Rink #2     Friday July 15 10:45:00 PM
Flaming Puke Buckets  Buffalo Team          Hockey Center - Rink #1     Friday July 22 07:15:00 PM
Swedish Fish          Buffalo Team          Hockey Center - Rink #2     Friday July 29 09:45:00 PM
```
