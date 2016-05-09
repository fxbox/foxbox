'use strict';

const FoxboxProcessManager = require('./foxbox_process_manager');
var foxboxProcessManager = new FoxboxProcessManager();

var testPrepperSelenium = (function() {

  function makeSuite(desc, test) {
    describe(desc, function () {
      this.timeout(30000);
      before(() => foxboxProcessManager.start());
      test();
      after(() => {
        foxboxProcessManager.kill();
        return foxboxProcessManager.cleanData();
      });
    });
  }

  return { makeSuite, foxboxProcessManager };
})();

module.exports = testPrepperSelenium;
