<Slide style={{ width: 1280, height: 720, background: '#FFFFFF', position: 'relative', padding: '48px 52px 68px 52px', flexDirection: 'column', gap: 24, fontFamily: 'Microsoft YaHei' }}>
    <svg width={140} height={140} viewBox='0 0 140 140' style={{ position: 'absolute', left: 1086, top: 16 }}>
        <rect x='0' y='0' width='140' height='140' fill='#FFFFFF' />
        <circle cx='70' cy='70' r='67' fill='none' stroke='#B5B8BB' strokeWidth='1' opacity='0.6' />
        <circle cx='70' cy='70' r='52' fill='none' stroke='#B5B8BB' strokeWidth='1' strokeDasharray='3 4' opacity='0.4' />
    </svg>

    <Box style={{ width: 1176, height: 80 }}>
        <Text style={{ fontSize: 13, fontWeight: 500, letterSpacing: 2.5, color: '#6D7176', lineHeight: 1, whiteSpace: 'nowrap' }}>
            〔章节标签〕
        </Text>
        <Text style={{ fontFamily: 'STSong', fontSize: 32, fontWeight: 700, lineHeight: 1.3, color: '#10141A', marginTop: 12, whiteSpace: 'nowrap' }}>
            〔标题〕
        </Text>
    </Box>

    <Box style={{ width: 1176, height: 500 }}>

        〔B区内容〕
    </Box>

    <Box style={{ position: 'absolute', left: 52, bottom: 28, height: 14 }}>
        <Text style={{ fontSize: 12, fontWeight: 500, letterSpacing: 2, lineHeight: 1, color: '#6D7176', whiteSpace: 'nowrap' }}>〔单位名〕</Text>
    </Box>
    <Box style={{ position: 'absolute', right: 52, bottom: 28, height: 14 }}>
        <Text style={{ fontFamily: 'Courier New', fontSize: 12, fontWeight: 400, letterSpacing: 2, lineHeight: 1, color: '#6D7176' }}>〔NN / 总页〕</Text>
    </Box>
</Slide>
