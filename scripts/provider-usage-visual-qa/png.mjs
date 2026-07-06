import { inflateSync } from "node:zlib"

export function inspectPng(buffer) {
  const png = parsePng(buffer)
  const distinct = new Set()
  let minLuma = 255
  let maxLuma = 0
  const pixelCount = png.width * png.height
  const stride = Math.max(1, Math.floor(pixelCount / 10_000))

  for (let pixel = 0; pixel < pixelCount; pixel += stride) {
    const offset = pixel * png.channels
    const red = png.pixels[offset] ?? 0
    const green = png.pixels[offset + 1] ?? red
    const blue = png.pixels[offset + 2] ?? red
    const alpha = png.channels === 4 ? png.pixels[offset + 3] ?? 255 : 255
    const luma = Math.round((red * 299 + green * 587 + blue * 114) / 1000)
    minLuma = Math.min(minLuma, luma)
    maxLuma = Math.max(maxLuma, luma)
    distinct.add(`${red},${green},${blue},${alpha}`)
  }

  return {
    width: png.width,
    height: png.height,
    sampledPixels: Math.ceil(pixelCount / stride),
    distinctSampledColors: distinct.size,
    luminanceRange: maxLuma - minLuma,
    nonblank: distinct.size >= 8 && maxLuma - minLuma >= 8
  }
}

function parsePng(buffer) {
  const signature = buffer.subarray(0, 8).toString("hex")
  if (signature !== "89504e470d0a1a0a") {
    throw new Error("Screenshot is not a PNG")
  }

  let offset = 8
  let width = 0
  let height = 0
  let bitDepth = 0
  let colorType = 0
  const idatChunks = []

  while (offset < buffer.length) {
    const length = buffer.readUInt32BE(offset)
    const type = buffer.subarray(offset + 4, offset + 8).toString("ascii")
    const data = buffer.subarray(offset + 8, offset + 8 + length)
    if (type === "IHDR") {
      width = data.readUInt32BE(0)
      height = data.readUInt32BE(4)
      bitDepth = data[8] ?? 0
      colorType = data[9] ?? 0
    }
    if (type === "IDAT") {
      idatChunks.push(data)
    }
    if (type === "IEND") {
      break
    }
    offset += length + 12
  }

  const channels = pngChannels(colorType)
  if (bitDepth !== 8 || channels === undefined || width <= 0 || height <= 0) {
    throw new Error(`Unsupported PNG shape: bitDepth=${bitDepth} colorType=${colorType}`)
  }

  const inflated = inflateSync(Buffer.concat(idatChunks))
  return { width, height, channels, pixels: unfilterPng(inflated, width, height, channels) }
}

function pngChannels(colorType) {
  switch (colorType) {
    case 0:
      return 1
    case 2:
      return 3
    case 6:
      return 4
    default:
      return undefined
  }
}

function unfilterPng(data, width, height, channels) {
  const rowBytes = width * channels
  const pixels = Buffer.alloc(rowBytes * height)
  for (let row = 0; row < height; row += 1) {
    const filter = data[row * (rowBytes + 1)]
    const sourceOffset = row * (rowBytes + 1) + 1
    const targetOffset = row * rowBytes
    for (let column = 0; column < rowBytes; column += 1) {
      const raw = data[sourceOffset + column] ?? 0
      const left = column >= channels ? pixels[targetOffset + column - channels] ?? 0 : 0
      const up = row > 0 ? pixels[targetOffset + column - rowBytes] ?? 0 : 0
      const upLeft = row > 0 && column >= channels ? pixels[targetOffset + column - rowBytes - channels] ?? 0 : 0
      pixels[targetOffset + column] = (raw + pngPredictor(filter, left, up, upLeft)) & 255
    }
  }
  return pixels
}

function pngPredictor(filter, left, up, upLeft) {
  switch (filter) {
    case 0:
      return 0
    case 1:
      return left
    case 2:
      return up
    case 3:
      return Math.floor((left + up) / 2)
    case 4:
      return paeth(left, up, upLeft)
    default:
      throw new Error(`Unsupported PNG filter: ${filter}`)
  }
}

function paeth(left, up, upLeft) {
  const estimate = left + up - upLeft
  const leftDistance = Math.abs(estimate - left)
  const upDistance = Math.abs(estimate - up)
  const upLeftDistance = Math.abs(estimate - upLeft)
  if (leftDistance <= upDistance && leftDistance <= upLeftDistance) {
    return left
  }
  return upDistance <= upLeftDistance ? up : upLeft
}
