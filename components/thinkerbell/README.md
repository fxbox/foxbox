# ThinkerBell: A scripting engine for the world of IoT

[![Build Status](https://api.travis-ci.org/fxbox/thinkerbell.svg?branch=master)](https://api.travis-ci.org/fxbox/thinkerbell)


These notes concentrate on Monitors, i.e. code that should executed on
the FoxBox.

For the time being, these scenarios are untriaged.

## Temperature setter

> During the day, temperature of the heater should be set to 19C, but
> during the night, reduce the temperature of heaters to 16C.

Can be expressed as
```
I need:
- `the current time` with property `time of day`;
- `heaters` (at least one), to `set temperature`.

1. When `time of day` of `the current time` increases beyond 7pm
	 do `set temperature` with `heaters` to 16C.
2. When `time of day` of `the current time` increases beyond 6am
    do `set temperature` with `heaters` to 19C.
```

For a simple script like this, the "I need" part can probably be
inferred from the actual triggers.


### Input devices
* `the current time` is a built-in pseudo device. For this example,
it needs the following capabilities:
	* `time of day` is, well, the time of day

Pseudo-devices need to be discovered and bound automatically, without
user interaction.

### Output devices
* `heaters` is a kind of device. In this example, the script applies
to any non-0 number of devices. For this example, it needs the
following capabilities:
   * `set temperature`, with a temperature

These are actual devices, which need a UX interaction to be bound to the script.


### Values
* Time of day. We probably want to represent it in military time
  (1900) and let the UX add syntax sugar to turn this into "7pm" in
  relevant countries.
* Temperature. We want units there (C or F, in particular), to avoid accidents.

### Operators
* `increases beyond` - first versions of this script used > and <, but
  we are actually more interested in *state change* than in current
  state. Note that this operator magically ensures that we do not
  saturate the heaters with requests to change state.

In the UX, we may wish to conflate state and state change. In the
interpreter, though, there is a big difference.

## Oven safety

> When I leave the house, if the oven is on, send me a message and
> sound a pre-recorded message on the speaker close to the door.

Can be expressed as
```
I need:
- a `presence monitor` with property `has presence`;
- a `oven` with property `is on`;
- a `communication channel to user` to `send text message`.

1. When `has presence` of `presence monitor` switches to false
    and `is on` of `oven` is false
    do `send text message` with `communication channel to user`: "Your left
    your oven on but there is nobody home."

```

### Input devices
* `presence monitor`, with property `has presence`
* `oven`, with property `is on`

### Output devices
* `communication channel to user` is a built-in pseudo device. It
  needs the FoxBox to be configured with access to the outside
  world. It has the following capabilities:
  * `send text message`

Implementing this may be difficult, since Web Push API are pretty much
not implemented on mobie devices.

### Values
* booleans

### Operators
* `switches to`, again measures a state change
* `is` measure a current state

Note that the order of execution of the branches in the AND will be
important to minimize energy use. We will want to be informed of
"switches to", rather than hammering "is".

## Light setter

> When the motion detector hasn't seen any movement in 10 minutes,
> turn off the lignts.

Can be expressed as
```
I need:
- `motion detectors` (at least one) with property `is there motion`;
- `lights` (at least one) to `turn off`;
- `10 minutes countdown` to `start`, `stop`, with property `is done`.

1. When `is there motion` of `motion detector` switches to false
do `start` with `10 minutes countdown`;
2. When `is there motion` of `motion detector` switches to true
do `stop` with `10 minutes countdown`;
3. When `is done` of `10 minutes countdown` switches to true
do `turn off` with `lights`.
```

Note that this does not require any explicit state.

### Input devices
* `motion detector`, with property `is there motion`;
* `10 minutes countdown`, built-in, with property `is done`.

### Output devices
* `lights`, with capability `turn off`;
* `10 minutes countdown` (the same one), with capabilities `start`, `stop`

Note that applications are sandboxed, so the 10 minutes countdown is
not visible by other applications.

### Values
Nothing new here.

## Home Security

> When I am on vacation, if my bedroom door opens, I want to receive a
> picture of whatever happened in my bedroom.

Can be expressed as
```
I need:
- a `door opening detector` with property `door is opened`;
- a `camera` with property `image`;
- a `communication channel to user` to `send text message` and `send image`.

1. When `door is opened` of `door opening detector` switches to true
    do `send text message` with `communication channel with user`:
       "Someone entered your bedroom"
    do `send image` with `communication channel with user`:
	  `image` of `camera`.
```


### Input devices
* `door opening detector`, with property `door is opened`;
* `camera`, with property `image`

The camera is a weird case, since property `image` can typically weigh several Mb.

### Output devices
* `communication channel with user` with capability `send image`


### Values
* The `image` is most likely a blob (binary data + mime type). The
rules engine doesn't know what to do with it.
* Regardless of how many times we access the value `image` of `camera`
during the evaluation of the trigger, this value is cached.
* Behind-the-scenes, requesting the `image` of `camera` is async. The
interpreter hides this from the user.


## Smart Device detector

> In this Highschool, when a wifi/broadband-enabled device enters or
> is turned on in the exam perimeter, send a message to the desktop
> server. Give as much detail as possible on where the device is, so
> that the teachers can come and frown at offending student.

Can be expressed as
```
I need:
- a `wifi detector` (zero or more) with property `activity detected`;
- a `broadband detector` (zero or more) with property `activity
detected`;
- a `communication channel to an application` with capability `send data``?

1. When `activity detected` of `wifi detector` switches to true
  do `send data` to `communication channel to an application`:
    `sensor details` of `activity detected`
```

This application is meant to be attached to a specific front-end.

### Input devices
* a `wifi detector` with property `activity detected`;
* a `broadband detector` with property `activity detected`.

### Output devices
* a `communication channel to an application` with capability `send data`;

FIXME: I'm not entirely sure about that one. This detector is meant to
be used with a specific front-end. What's the best way to specify
front-end? Do we want to be so generic that we can change front-end?
That sounds over-engineered, but I can't think of anything simpler atm.


## Pollution monitor

> If any pollution sensor detects more than n1% of CO2 or n2% of CO,
> etc. send a secure message to a web service.

TBD

## Humidity detector

> If the server room is humid, inform all admins.

TBD

## Supply management

> If there are no more cookies on the shelves of the store, send a
> message to the manager. Don't do this more than once per hour.

TBD

## Art museum painting protectors

> Light sensors detect use of flash. If a flash is detected, ring an
> annoying sound.

TBD

# How can we handle a server upgrade?

We may need to save some state on behalf of the apps.

TBD

## See also

* [More applications of sensors ](http://www.libelium.com/top_50_iot_sensor_applications_ranking/)
* [More applications](https://temboo.com/iot-applications)
* [And on Wikipedia](https://en.wikipedia.org/wiki/Internet_of_Things#Applications)
