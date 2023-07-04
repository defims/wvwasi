;(function() {
  var origin = 'https://wvwasi.localhost'

  function WvWasi() {
    this.wasiSnapshotPreview1 = new WasiSnapshotPreview1()
  }

  window.WvWasi = WvWasi

  function WasiSnapshotPreview1() {}

  WasiSnapshotPreview1.prototype.init = function() {
    var that = this
    return Promise.all([
      fetch(`${origin}/wasi_snapshot_preview1/init`).then(x => x.json()),
      new Promise(resolve => {
        const sharedBufferReceivedHandler = e => {
          resolve(e.getBuffer())
          window.chrome.webview.removeEventListener("sharedbufferreceived", sharedBufferReceivedHandler);
        }
        window.chrome.webview.addEventListener("sharedbufferreceived", sharedBufferReceivedHandler);
      })
    ]).then(function(result) {
      var json = result[0] || []
      that.wvwasi = json[1]
      that.rootFd = json[2]
      that.sharedBuffer = result[1]
      that.sharedBufferPtr = 0
      return that
    })
  }

  WasiSnapshotPreview1.prototype.fd_readdir = function(fd, buf, buf_len, cookie, bufused) {
    var that = this;
    return fetch(
      `${origin}/${that.wvwasi}/wasi_snapshot_preview1/fd_readdir`,
      {
        method: 'POST',
        body: JSON.stringify([ fd, buf, buf_len, cookie, bufused ])
      }
    ).then(function(x) {
      return x.json()
    }).then(function(x) {
      if(x[0] === 0) {
        const bufused_value = new Int32Array(that.sharedBuffer.slice(bufused, bufused + 4))[0]
        that.sharedBufferPtr = Math.max(that.sharedBufferPtr, bufused + bufused_value + 1, buf + 1)
        return bufused_value
      } else {
        throw new Error("fd_readdir error")
      }
    })
  }

  WasiSnapshotPreview1.prototype.path_remove_directory = function(fd, path) {
    var that = this;
    return fetch(
      `${origin}/${that.wvwasi}/wasi_snapshot_preview1/path_remove_directory`,
      {
        method: 'POST',
        body: JSON.stringify([ fd, path ])
      }
    ).then(function(x) {
      return x.json()
    }).then(function(x) {
      if(x[0] !== 0) {
        throw new Error("path_remove_directory error")
      }
    })
  }

  WasiSnapshotPreview1.prototype.path_unlink_file = function(fd, path) {
    var that = this;
    return fetch(
      `${origin}/${that.wvwasi}/wasi_snapshot_preview1/path_unlink_file`,
      {
        method: 'POST',
        body: JSON.stringify([ fd, path ])
      }
    ).then(function(x) {
      return x.json()
    }).then(function(x) {
      if(x[0] !== 0) {
        throw new Error("path_unlink_file error")
      }
    })
  }
})();