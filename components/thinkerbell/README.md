# ThinkerBell: A scripting engine for the world of IoT

These notes concentrate on Monitors, i.e. code that should executed on
the FoxBox.

For the time being, these scenarios are untriaged.

## Temperature setter

> During the day, temperature of the heater should be set to 19C, but
> during the night, reduce the temperature of heaters to 16C.

Can be expressed as
```
I need:
- `the current time` to `get time of day`;
- `heaters` (at least one), to `set temperature`.

1. When `time of day` of `the current time` increases beyond 7pm
	 do `set temperature` of `heaters` to 16C.
2. When `time of day` of `the current time` increases beyond 6am
    do `set temperature` of `heaters` to 19C.
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

Output device:
* all heaters (not a single IoT device, rather a set of devices).

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


## Oven safety

> When I leave the house, if the oven is on, send me a message and
> sound a pre-recorded message on the speaker close to the door.

Input devices:
* something that will tell the FoxBox that nobody is home. Perhaps an door-opened detector. Perhaps the owner's cellphone;
* the oven's on/off state.

Output devices:
* message sender (using Firefox Accounts rather than a real IoT
  device, but we should be able to behave as if it was);
* device that can play sound.

Additional note:
* do we want to send an entire sound file to the sound-playing device?

## Light setter

> When the motion detector hasn't seen any movement in 10 minutes,
> turn off the lignts.

This one actually needs several triggers.

1. When the motion detector stops seeing movement, start 10 minutes countdown.
2. When the motion detector starts seeing movement, stop 10 minutes countdown.
3. When 10 minutes countdown complete, turn off the lights.

Input devices:
* motion detector (start/stop);
* 10 minutes countdown (complete).

Output device:
* 10 minutes countdown (start/stop);
* all lights (not a single IoT device, rather a set of devices).

Additional notes:
* since the 10 minute countdown is a pseudo-device, we don't need a
  special way to identify it;
* apps are sandboxed, so the countdown can only be seen by this app;
* we do not need to store any state in the app.

## Smart Device detector

> In this Highschool, when a wifi/broadband-enabled device enters or
> is turned on in the exam perimeter, send a message to the desktop
> server. Give as much detail as possible on where the device is, so
> that the teachers can come and frown at offending student.

Input devices:
* wifi detectors;
* broadband detectors.

Output device:
* message sender;

Additional notes:
* we want to be able to send data on *which* sensor informed us;
* we need to be able to send messages.

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
