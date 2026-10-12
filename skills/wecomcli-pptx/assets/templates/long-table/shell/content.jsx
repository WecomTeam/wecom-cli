<Slide style={{ width: 1280, height: 720, background: '#FFFFFF', padding: '50px 64px 0 64px', position: 'relative', fontFamily: 'STSong' }}>
    <svg width={1280} height={720} viewBox='0 0 1280 720' style={{ position: 'absolute', top: 0, left: 0 }}>
        <defs>
            <pattern id='dots' width='6' height='6' patternUnits='userSpaceOnUse'>
                <circle cx='3' cy='3' r='0.7' fill='#FBF1E0' />
            </pattern>
        </defs>
        <rect x='0' y='0' width='1280' height='720' fill='#FFFFFF' />
        <rect x='0' y='0' width='1280' height='720' fill='url(#dots)' />
    </svg>

    <Box style={{ position: 'relative', flexDirection: 'row', justifyContent: 'space-between', alignItems: 'flex-end', height: 84, paddingBottom: 14, borderBottom: '1.5px solid #C42819' }}>
        <Box style={{ flexDirection: 'row', alignItems: 'center', gap: 16 }}>
            <Box style={{ width: 10, height: 44, background: '#C42819' }} />
            <Text style={{ fontSize: 46, fontWeight: 700, lineHeight: 1.2, color: '#1A1616', whiteSpace: 'nowrap' }}>〔标题〕</Text>
        </Box>
        <Box style={{ flexDirection: 'row', alignItems: 'center', gap: 14, marginBottom: 6 }}>
            <Box style={{ flexDirection: 'row', gap: 8 }}>
                <Box style={{ width: 8, height: 8, borderRadius: 4, background: '#E19825' }} /><Box style={{ width: 8, height: 8, borderRadius: 4, background: '#E19825', opacity: 0.6 }} /><Box style={{ width: 8, height: 8, borderRadius: 4, background: '#E19825', opacity: 0.3 }} />
            </Box>
            <Text style={{ fontSize: 15, color: '#6B7280', lineHeight: 1.2 }}>〔整份标题 · 年份 · 页名〕</Text>
        </Box>
    </Box>

    <Box style={{ position: 'relative', marginTop: 18, width: 1152, height: 488 }}>

        〔B区内容〕
    </Box>

    <Box style={{ position: 'absolute', left: 0, top: 660, width: 1280, height: 9, flexDirection: 'row' }}>
        <Box style={{ width: 678, height: 9, background: '#C42819' }} />
        <Box style={{ width: 333, height: 9, background: '#EE5530' }} />
        <Box style={{ width: 269, height: 9, background: '#F26B40' }} />
    </Box>
    <Text style={{ position: 'absolute', right: 64, top: 678, fontFamily: 'Times New Roman', fontSize: 15, fontStyle: 'italic', color: '#6B7280', lineHeight: 1 }}>〔NN / 总页〕</Text>
</Slide>
