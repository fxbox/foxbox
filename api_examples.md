# API Usage Examples

All the requests have to be authenticated unless you compiled Foxbox with
authentication disabled.

## To change light status:

`PUT` to `api/v1/channels/set` :

```json
{ "select": {
    "id": "channel:power.1.001788fffe251236.philips_hue@link.mozilla.org",
    "feature": "light/is-on"
  },
  "value": "Off"
}
```

## To retrieve light status:

`PUT` to `api/v1/channels/get` :

```json
{
  "id": "channel:power.1.001788fffe251236.philips_hue@link.mozilla.org", 
  "feature": "light/is-on"
}
```

## To say something:

`PUT` to `api/v1/channels/set` :

```json
{
  "select": {
    "id": "setter:talk@link.mozilla.org",
    "feature": "speak/sentence"
  },
  "value": "Hello FoxBox"
}
```