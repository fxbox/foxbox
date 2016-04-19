/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

/* export getElementName */

'use strict';

function getElementName(str) {
  str = str.toLowerCase().replace('#', '');
  return str.replace(/-([a-z])/g, function(g) {
    return g[1].toUpperCase();
  });
}
