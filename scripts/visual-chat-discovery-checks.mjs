export async function verifyVisualState(page, state, width) {
  await assertNoHorizontalOverflow(page, state, width)
  const markerCounts = await assertQaMarkers(page, state, width)
  const textLayout = await assertTextLayout(page, state, width)
  const focusChecks = await assertFocusChecks(page, state, width)
  const hoverChecks = await assertHoverChecks(page, state, width)
  const previewVisibility = await assertPreviewVisibility(page, state, width)
  return { markerCounts, textLayout, focusChecks, hoverChecks, previewVisibility }
}

async function assertNoHorizontalOverflow(page, state, width) {
  const overflow = await page.evaluate(() => ({
    scrollWidth: document.documentElement.scrollWidth,
    innerWidth: window.innerWidth
  }))
  if (overflow.scrollWidth > overflow.innerWidth) {
    throw new Error(
      `${state}@${width}: horizontal overflow ${overflow.scrollWidth}px > ${overflow.innerWidth}px`
    )
  }
}

async function assertQaMarkers(page, state, width) {
  const markerCounts = await page.evaluate(() => ({
    text: document.querySelectorAll("[data-visual-qa-text]").length,
    control: document.querySelectorAll("[data-visual-qa-control]").length
  }))
  if (markerCounts.text < 1 || markerCounts.control < 1) {
    throw new Error(
      `${state}@${width}: missing QA markers text=${markerCounts.text} control=${markerCounts.control}`
    )
  }
  return markerCounts
}

async function assertTextLayout(page, state, width) {
  const problems = await page.evaluate(() => {
    const elements = Array.from(document.querySelectorAll("[data-visual-qa-text]"))
    return elements.flatMap((element) => {
      const label = element.getAttribute("data-visual-qa-text") ?? element.tagName
      const rect = element.getBoundingClientRect()
      const style = window.getComputedStyle(element)
      const problemsForElement = []
      if (
        rect.width <= 0 ||
        rect.height <= 0 ||
        style.display === "none" ||
        style.visibility === "hidden"
      ) {
        problemsForElement.push(`${label} has no positive visible box`)
      }
      if (rect.left < -1 || rect.right > window.innerWidth + 1) {
        problemsForElement.push(
          `${label} is clipped horizontally (${rect.left.toFixed(1)}-${rect.right.toFixed(1)})`
        )
      }

      const next = nextVisualSibling(element)
      if (next !== undefined && boxesIntersect(rect, next.getBoundingClientRect())) {
        const nextLabel =
          next.getAttribute("data-visual-qa-text") ??
          next.getAttribute("data-visual-qa-control") ??
          next.tagName
        problemsForElement.push(`${label} overlaps next visual sibling ${nextLabel}`)
      }
      return problemsForElement
    })

    function nextVisualSibling(element) {
      let sibling = element.nextElementSibling
      while (sibling !== null) {
        if (
          sibling.hasAttribute("data-visual-qa-text") ||
          sibling.hasAttribute("data-visual-qa-control")
        ) {
          return sibling
        }
        sibling = sibling.nextElementSibling
      }
      return undefined
    }

    function boxesIntersect(left, right) {
      return (
        left.left < right.right - 1 &&
        left.right > right.left + 1 &&
        left.top < right.bottom - 1 &&
        left.bottom > right.top + 1
      )
    }
  })

  if (problems.length > 0) {
    throw new Error(`${state}@${width}: text layout problems:\n${problems.join("\n")}`)
  }
  return { checkedTextBoxes: await page.locator("[data-visual-qa-text]").count() }
}

async function assertFocusChecks(page, state, width) {
  const controls = page.locator("[data-visual-qa-control]")
  const count = await controls.count()
  let focusableCount = 0

  for (let index = 0; index < count; index += 1) {
    const control = controls.nth(index)
    if (await control.evaluate((element) => element.matches(":disabled"))) {
      continue
    }
    await control.focus()
    const result = await control.evaluate((element) => {
      const style = window.getComputedStyle(element)
      return {
        focused: document.activeElement === element,
        outlineStyle: style.outlineStyle,
        outlineWidth: style.outlineWidth
      }
    })
    if (!result.focused) {
      throw new Error(`${state}@${width}: QA control ${index} could not receive focus`)
    }
    if (result.outlineStyle === "none" || result.outlineWidth === "0px") {
      throw new Error(`${state}@${width}: QA control ${index} has no visible focus outline`)
    }
    focusableCount += 1
  }

  const keyboardFocusedMarker = await keyboardCanReachQaControl(page)
  if (focusableCount === 0 || !keyboardFocusedMarker) {
    const disabledControlCount = await controls.evaluateAll((elements) =>
      elements.filter((element) => element.matches(":disabled")).length
    )
    if (disabledControlCount === count) {
      return { focusableCount, keyboardFocusedMarker, disabledControlCount }
    }
    throw new Error(`${state}@${width}: keyboard focus did not reach a QA control`)
  }
  return { focusableCount, keyboardFocusedMarker, disabledControlCount: count - focusableCount }
}

async function keyboardCanReachQaControl(page) {
  await page.evaluate(() => {
    if (document.activeElement instanceof HTMLElement) {
      document.activeElement.blur()
    }
  })
  for (let attempt = 0; attempt < 16; attempt += 1) {
    await page.keyboard.press("Tab")
    const focusedMarker = await page.evaluate(() =>
      document.activeElement?.hasAttribute("data-visual-qa-control") ?? false
    )
    if (focusedMarker) {
      return true
    }
  }
  return false
}

async function assertHoverChecks(page, state, width) {
  const syncHover = await assertHoverable(
    page,
    page.locator('[data-visual-qa-control="sync-now"]'),
    `${state}@${width}: Sync Now`,
    expectedSyncNowState(state)
  )
  const selectedRows = page.locator('[data-visual-qa-row="selected-chat"]')
  let selectedRowHover = "not-present"
  if ((await selectedRows.count()) > 0) {
    selectedRowHover = await assertHoverable(
      page,
      selectedRows.first(),
      `${state}@${width}: selected chat row`,
      "any"
    )
  }
  return { syncHover, selectedRowHover }
}

async function assertPreviewVisibility(page, state, width) {
  const previews = page.locator('[data-visual-qa-text="chat-row-preview"]')
  const count = await previews.count()
  const sampleText = count > 0 ? normalizeText(await previews.first().innerText()) : ""

  if (state === "ready-previews-revealed") {
    if (count === 0 || sampleText.length === 0) {
      throw new Error(`${state}@${width}: expected revealed preview text`)
    }
    return { expected: "present", count, sampleText }
  }

  if (count > 0) {
    throw new Error(`${state}@${width}: preview text must be hidden, found ${count} row previews`)
  }
  return { expected: "absent", count, sampleText }
}

function normalizeText(value) {
  return value.replace(/\s+/g, " ").trim()
}

function expectedSyncNowState(state) {
  if (state === "ready-selected") {
    return "enabled"
  }
  return "disabled"
}

async function assertHoverable(page, locator, label, expectedState) {
  if ((await locator.count()) === 0) {
    throw new Error(`${label} is missing`)
  }
  const before = await visualHoverStyle(locator)
  const disabled = await locator.evaluate((element) => element.matches(":disabled"))
  await locator.hover()
  await page.waitForTimeout(180)
  const hovered = await locator.evaluate((element) => element.matches(":hover"))
  if (!hovered) {
    throw new Error(`${label} did not enter :hover`)
  }
  const after = await visualHoverStyle(locator)
  if (!disabled && before === after) {
    throw new Error(`${label} hover produced no visual style change`)
  }
  if (expectedState === "enabled" && disabled) {
    throw new Error(`${label} was disabled but should be enabled`)
  }
  if (expectedState === "disabled" && !disabled) {
    throw new Error(`${label} was enabled but should be disabled`)
  }
  return disabled ? "hovered-disabled-control" : "hovered-with-style-change"
}

async function visualHoverStyle(locator) {
  return locator.evaluate((element) => {
    const style = window.getComputedStyle(element)
    return `${style.backgroundColor}|${style.borderColor}|${style.color}|${style.transform}`
  })
}
