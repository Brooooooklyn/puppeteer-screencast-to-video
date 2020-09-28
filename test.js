const { readFileSync, writeFileSync } = require('fs')
const { join } = require('path')

const { encode } = require('./index')

const framesMeta = readFileSync(join(__dirname, 'screencast.json'))

const meta = JSON.parse(framesMeta)

const buf = Buffer.from(meta.frames[0].data, 'base64')
console.log(buf.length)

encode(framesMeta).then((buf) => {
  writeFileSync(join(__dirname, 'screencast.mp4'), buf)
})
