---
name: component-codeblock
description: CodeBlock component - syntax highlighting, language, theme, macOS header
---

# CodeBlock 组件规范

CodeBlock 组件用于展示语法高亮的代码块，支持多种编程语言，macOS 风格设计。

## 核心属性

- **code**: 代码字符串
- **language**: 编程语言，如 `"javascript"`, `"python"`, `"typescript"`
- **width**: 宽度（像素）
- **fontSize**: 字体大小（默认 12）
- **theme**: `"github-dark"` 或 `"github-light"`（默认 `"github-dark"`）
- **macHeader**: 是否显示 macOS 三色点（默认 true）

## 示例代码

### 1. 基础 JavaScript 代码

```jsx
<CodeBlock
    code={`function fibonacci(n) {
  if (n <= 1) return n;
  return fibonacci(n - 1) + fibonacci(n - 2);
}

console.log(fibonacci(10)); // 55`}
    language='javascript'
    width={700}
/>
```

### 2. Python 代码

```jsx
<CodeBlock
    code={`def quick_sort(arr):
    if len(arr) <= 1:
        return arr
    pivot = arr[len(arr) // 2]
    left = [x for x in arr if x < pivot]
    middle = [x for x in arr if x == pivot]
    right = [x for x in arr if x > pivot]
    return quick_sort(left) + middle + quick_sort(right)`}
    language='python'
    width={800}
    fontSize={14}
/>
```

### 3. TypeScript 代码（明亮主题）

```jsx
<CodeBlock
    code={`interface User {
  id: string;
  name: string;
}

class UserService {
  private users: Map<string, User> = new Map();
  
  addUser(user: User): void {
    this.users.set(user.id, user);
  }
}`}
    language='typescript'
    width={750}
    theme='github-light'
    background='#ffffff'
/>
```
