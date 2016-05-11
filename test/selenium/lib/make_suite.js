'use strict';

const FoxboxProcessManager = require('./foxbox_process_manager');
const App = require('./setup_webapp');

function makeSuite(description, hostUrl, subSuite) {

  if (typeof hostUrl === 'function') {
    subSuite = hostUrl;
    hostUrl = FoxboxProcessManager.HOST_URL;
  }

  var foxboxProcessManager = new FoxboxProcessManager();
  var app = new App(hostUrl);

  describe(description, function () {
    this.timeout(30000);

    before(() => foxboxProcessManager.start());

    beforeEach(() => app.init());

    subSuite(app);

    after(() => {
      return app.stop().then(() => {
        foxboxProcessManager.kill();
        return foxboxProcessManager.cleanData();
      });
    });
  });
}


module.exports = makeSuite;
